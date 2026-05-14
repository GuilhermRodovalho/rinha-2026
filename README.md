# Rinha de Backend 2026 — Fraud Score

API de detecção de fraude em tempo real baseada em k-NN sobre um índice IVF construído a partir de 3 milhões de transações rotuladas. Feito como submissão para a [rinha de backend 2026](https://github.com/zanfranceschi/rinha-de-backend-2026).
Usei a stack de Rust, Axum e Tokio, com a ideia de aplicar um pouco dos conhecimentos obtidos no [Zero to Production in Rust](https://www.zero2prod.com/index.html?country=Brazil&discount_code=SA60)
Usei de bastante inspiração a [submissão feita pelo jairoblatt](https://github.com/jairoblatt/rinha-2026-rust). 


## Como funciona

A requisição chega com os dados da transação, que são convertidos em um vetor de 14 dimensões (`f32`). Esse vetor é comparado contra o índice IVF para encontrar os 5 vizinhos mais próximos e calcular a proporção de fraudes entre eles. O resultado determina a pontuação e a aprovação.

## Pipeline de construção

O índice é pré-computado offline por dois binários separados do servidor:

1. `build_bin` — lê `data/references.json` e serializa os vetores e labels em um formato binário compacto (`data/raw_data.bin`), usando um deserializador de streaming para evitar manter o JSON inteiro na memória. (desnecessário, fiz como um primeiro passo para aprendizado e tentar escrever um arquivo binário)

2. `build_index` — lê o `.bin`, executa K-Means com K=2048 e escreve o índice IVF em `data/index.bin.gz`. O K-Means usa paralelismo por threads para a etapa de assignment (a mais custosa) e encerra cedo se o deslocamento máximo dos centroides cair abaixo de `1e-4`.

O servidor não executa nenhuma dessas etapas — ele só carrega o índice já pronto.

## Decisoes de performance

### Indice IVF em vez de busca linear

Uma busca linear sobre 3M vetores de 14 dimensoes custaria ~42M operacoes de ponto flutuante por requisicao. O IVF (Inverted File Index) divide o espaco em K=2048 clusters via K-Means. Na hora da query, apenas `nprobe=8` (no melhor caso) clusters sao varridos, o que reduz o numero de distancias calculadas para ~1% do total.

### Indice embutido no binario (`include_bytes!`)

O arquivo `index.bin.gz` e incorporado diretamente no binario do servidor via `include_bytes!`. Isso elimina qualquer I/O de disco no caminho quente. O carregamento inicial descomprime o arquivo uma unica vez na subida do processo.

O arquivo de dados brutos (`raw_data.bin`, ~163 MB) nao é embutido — ele é lido do disco apenas pelo `build_index`, que nao é o binário de produção.

### Quantizacao i16

Os vetores no indice sao armazenados como `i16` com fator de escala `SCALE=10_000`, em vez de `f32`. Isso reduz o consumo de memoria dos vetores de ~168 MB para ~84 MB (3M x 14 x 2 bytes). A dequantizacao na hora do calculo e uma multiplicacao por `1/SCALE`, sem perda relevante de precisão para os dados normalizados entre 0 e 1.

### `select_nth_unstable` no lugar de sort

Para encontrar os `nprobe` clusters mais proximos entre os K=2048 centroides, o codigo usa `select_nth_unstable_by`, que tem complexidade O(K) em vez de O(K log K) do sort completo. Como so os indices dos primeiros `nprobe` elementos importam (nao a ordem entre eles), o sort seria trabalho desnecessario.

### Top-k na stack

Os 5 vizinhos mais proximos sao mantidos em um array fixo `[(f32, u8); 5]` alocado na stack, sem nenhuma alocacao no heap durante o scan. Como k=5 e estatico, o tamanho e conhecido em tempo de compilacao.

### nprobe adaptativo

Se o resultado com `nprobe=8` retornar 2 ou 3 fraudes entre os 5 vizinhos (zona de incerteza), a busca e repetida com `nprobe=24` para maior confiabilidade. Casos claros (0-1 ou 4-5 fraudes) nao pagam esse custo extra.

### `spawn_blocking` para CPU-bound

A busca k-NN e trabalho CPU-bound que bloquearia o executor async do Tokio. Ela e executada via `tokio::task::spawn_blocking`, liberando as threads async para continuar recebendo requisicoes durante o calculo.

### `worker_threads = 2`

O runtime do Tokio e configurado com apenas 2 threads de trabalho, compatível com o budget de CPU da instancia (0.45 vCPUs). Ter mais threads do que CPUs disponiveis so adicionaria overhead de troca de contexto.

## Melhorias futuras 
- Carregar os vetores em lote, para fazer uso do SIMD, fazendo a mesma operação em muitos dados diferentes
- Testar K=2048 e mudanças no nprobe, como caso base=12, ou algo assim (no estado atual (as of 14/05/2026), ainda está dando 3 falsos negativos, e 3 falsos positivos dentre todos os casos testes).
- Testar outra configuração de loadbalancer, ou outro runtime async no lugar do Tokio, ou outro framework (ou nenhum) no lugar do Axum.


## Infraestrutura

O `docker-compose.yml` sobe duas instancias identicas da API (`api1` e `api2`), cada uma com limite de 0.45 vCPUs e 170 MB de memoria. O nginx faz o balanceamento entre elas com `keepalive 32`, mantendo conexoes HTTP/1.1 persistentes para evitar o overhead de TCP handshake a cada requisicao.

```
nginx  (0.10 vCPU, 10 MB)
  |-- api1 (0.45 vCPU, 170 MB)
  |-- api2 (0.45 vCPU, 170 MB)

Total: 1.00 vCPU, 350 MB
```

## Rotas

| Metodo | Rota          | Descricao                         |
|--------|---------------|-----------------------------------|
| POST   | /fraud-score  | Calcula score e aprova/recusa     |
| GET    | /ready        | Healthcheck                       |
