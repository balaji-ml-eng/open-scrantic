# Como o Johnny Castaway funciona

Notas da engenharia reversa do `SCRANTIC.SCR` (executável Win16 NE, Borland C++ 1991, 13 segmentos
de código + DGROUP). Endereços no formato `segmento:offset`; `DS:xxxx` é o segmento de dados.

## Arquivos de recursos

- `RESOURCE.MAP`: cabeçalho de 6 bytes, nome do volume (13 bytes), `u16` contagem, entradas
  `(u32 hash, u32 offset)`. No `RESOURCE.001`, cada recurso: nome (13 bytes), `u32` tamanho, dados.
- Cada recurso é uma sequência de *chunks* `TAG:` + `u32` tamanho (bit 31 = contêiner).
- Compressão (`BIN:`, `SCR:`, `TT3:`): byte de método + `u32` tamanho; 1 = RLE, 2 = LZW
  (códigos de 9 a 12 bits LSB-first; o código 0x100 reinicia o dicionário e realinha o fluxo).
- Imagens em 4 bpp (nibble alto primeiro); índice 0 (magenta) é transparente. Os chunks `VGA:`
  são irrelevantes (ruído); a paleta `JOHNCAST.PAL` só usa 16 cores (6 bits por canal).

## TTM: scripts de animação (`seg12:040c`)

- Instruções `u16`: o nibble baixo é o número de argumentos `i16` (0xF = string terminada em NUL,
  preenchida até tamanho par). O script é dividido em *frames* em cada `0x0FF0` (UPDATE); `PAG:`
  dá o número de frames.
- Cada `0x1111 tag` define uma **cena** começando naquele frame. `0x1201` (GOTO) resolve para o
  frame cujo **primeiro** opcode é `0x1101/0x1111` com aquela tag.
- Um frame executado pela primeira vez tem efeitos colaterais (carregar imagens/telas, sons,
  atraso aleatório); depois só redesenha (flag `+0x0E` do registro da cena).
- Opcodes usados: sprites `0xA5x4` (bits 4–7 = espelhamento: 2 horizontal, 1 vertical), linha
  `0xA0A4` (sem o ponto final, como `LineTo`), retângulo `0xA104`/`0xA114`, elipse
  `0xA404/0xA424` (raio = h/2), pixel `0xA002`, clip `0x4004` (inclusivo), "copiar para o fundo"
  `0x4204`, atraso `0x1021` (ticks de 16 ms), atraso aleatório `0x2022` (`a + rand % (b - a)`),
  som `0xC051`, fim de cena `0x0110`, salvar/restaurar zona `0xA054/0xA064`.
- Coordenadas de desenho, clip e cópia recebem o deslocamento da ilha (`origem.x + 1`, `origem.y`),
  mais 272 px para cenas com a flag `0x400`.

## ADS: sequenciamento (`seg10`)

- Seções (tags) separadas por `0xFFFF`. Linguagem em blocos: condições `0x13x0`
  (`0x1330` nunca tocada, `0x1350` acabou de terminar, `0x1360` parada, `0x1370` rodando),
  `AND 0x1420`/`OR 0x1430` avaliados da esquerda para a direita com curto-circuito,
  `ELSE 0x1500`, `ENDIF 0x1510`, `RANDOM 0x3010 … 0x30FF` (pesos; `0x3020` = não fazer nada).
- `0x2005 slot tag n`: inicia uma cena (`n=0` uma vez, `n>0` n vezes, `n<0` por −n ticks);
  `0x2010` para; `0x4000/0x4010` traz a cena para frente/fundo; `0xF010` para uma tag;
  `0xF200` inicia outra tag em paralelo.
- Escalonador, a cada passo: propaga o estado das tags às suas cenas, **reavalia o script inteiro
  de cada tag ativa**, e então executa (desenha) o frame atual de cada cena na ordem de desenho,
  avançando quando o atraso venceu. Uma tag "acabou" quando nenhuma de suas cenas ainda roda
  (cenas que fazem GOTO para o próprio frame contam como laço infinito e não bloqueiam).

## História (`seg4`)

- Catálogo de 80 cenas em `DS:1756` (17 bytes): peso, pontos/direções de início e fim, duração,
  janela de fase do dia, dia da história, flags (papel: 1 início, 2 meio, 4 final, 8 repetível;
  modificadores da ilha: maré, noite, sem ilha, sem jangada, ondas congeladas; restrições de
  posição da ilha), id da ADS e tag. Cenas com ADS `0xFF` são pausas em pé (3–7 s).
- Cada **sessão** escolhe: com 10% de chance o capítulo do dia, uma cena final, uma de abertura e
  cenas de meio sorteadas por peso até gastar um orçamento de 300 unidades de duração, respeitando
  a compatibilidade com a posição da ilha já decidida.
- **Ciclo dia/noite**: 16 meias-horas a partir de `StartTime` (9:00 por padrão): 12 de dia
  (`OCEAN0x.SCR` aleatório) e 4 de noite (`NIGHT.SCR`).
- **Capítulos** (11): `NumDays` avança um por dia de calendário, desde que o capítulo atual já tenha
  sido mostrado; depois do 11º recomeça. A jangada cresce com o dia (5 estágios).
- **Feriados**: Halloween (29–31/10), Natal (23–25/12), Ano-novo (29/12–1/1), St. Patrick (15–17/3).

## Caminhada

Seis pontos na ilha; rotas por par (origem, destino) em `DS:0A96` com caminhos alternativos
sorteados por porcentagem; 34 caminhos de frames `(sprite|espelho, x, y)`; giro pelo menor lado
(aleatório a 180°). Quando os pés de Johnny estão acima de `origem.y + 293` e ele cruza o tronco,
o tronco é redesenhado na frente dele.

## Tempo e aleatoriedade

- Tick de 16 ms (`4 × 1000/(13×18)` ms) para atrasos e temporizadores; o passo da história roda no
  `WM_TIMER` de 50 ms (granularidade real de 55 ms no Windows 3.1). Ambos são emulados.
- Gerador aditivo com atraso sobre uma tabela de 56 palavras (`DS:1CE6`), aquecido descartando
  `time() & 0xFFFF` números.
- Sons: recursos `WAVESFX1..25` (faltam 11 e 13), 8 bits, 11 025 Hz; um som novo corta o anterior.
  `WAVESFX25` é o sino tocado quando começa o capítulo do dia.

## Diferenças deliberadas em relação ao original

- O original desenha com retângulos sujos em três buffers; aqui cada passo recompõe o quadro
  inteiro (fundo + desenhos do passo). O resultado é o mesmo, exceto pequenos artefatos do
  original (ex.: ondas da praia "voltando" ao primeiro quadro quando algo passa por cima).
- A tela título fica 2 s (no hardware de 1993 ela ficava visível enquanto a primeira sessão carregava).
- Elipses e linhas usam rasterização própria, que pode diferir de 1 px da do GDI.
- Diálogo de configuração e senha do protetor de tela não foram reimplementados (as opções são
  editáveis no INI). Recursos não usados por este jogo (paletas animadas, 256 cores, variáveis ADS)
  não foram implementados.
