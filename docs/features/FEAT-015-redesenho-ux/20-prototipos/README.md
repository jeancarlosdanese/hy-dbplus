# @file: docs/features/FEAT-015-redesenho-ux/20-prototipos/README.md

# Protótipo: FEAT-015

Um arquivo só, `prototipo.html`, aberto direto no navegador (sem servidor e sem rede). Dados fictícios. Fica fora
do workspace Rust e do build. No topo há o seletor de tela, as setas (também ← e → do teclado), o tema e o idioma.
O endereço guarda a tela: `prototipo.html#s=8&t=dark&l=en`.

| Fluxo | Artifact ou arquivo | Status |
|---|---|---|
| Todas as telas, claro e escuro, pt-BR e en | `prototipo.html` | aprovado pelo Jean (17/11/2026); implementado nas fases B a E |

## Telas

1. Conexões.
2. Conexões: lista vazia.
3. Formulário de conexão.
4. Remover conexão.
5. Sem banco aberto.
6. Paleta ⌘K.
7. Dados.
8. Dados: editando, com o painel de registro.
9. Gravar alterações.
10. Estrutura.
11. Índices.
12. DDL.
13. Editor SQL com resultados.
14. Editor SQL: autocomplete e execução.
15. Editor SQL: erro e transação abortada.
16. Fechar com pendências.
17. Sistema visual: cores, botões, tipos, espaçamento e estados.

## Decisões visuais

Propostas e aprovadas em 17/11/2026.

- **Destaque:** azul-petróleo da marca (`#1C5F72` no claro, `#5DBBD0` no escuro). Uma ação principal por tela.
- **Ambiente:** a barra superior deixa de ser pintada inteira. O ambiente aparece como etiqueta com texto e como
  faixa de 3 px sob a barra. Dev vira violeta e Prod um vermelho-alaranjado, para não colidir com destaque e erro.
- **Alteração pendente:** âmbar, com marca na borda da célula; seleção continua azul. Exclusão em vermelho riscado.
- **Aviso e erro:** aviso em âmbar (transação aberta), erro em vermelho, sempre em faixa com ícone.
- **Ícones:** uma família de traço, no lugar dos caracteres e do emoji.
- **Tipos:** Inter na interface (a mesma da logomarca), fonte de largura fixa em dados e SQL. Tamanhos 20, 16, 13
  e 12.
- **Medidas:** espaçamento em 4, 8, 12, 16 e 24. Linha de grid 24, barras 40, rodapé 32. Cantos 6, 8 e 12.
- **Barras:** as ações da tabela sobem para uma barra acima do grid; o rodapé fica com pendências e paginação.
- **Diálogos:** todos modais, com fundo escurecido, título que diz o que acontece, cancelar à esquerda da ação
  principal e as duas à direita. Ação destrutiva em vermelho.
- **Idioma:** seletor PT | EN na barra superior, entre a busca ⌘K e o botão do painel, e no cabeçalho da tela de
  conexões. Em texto, sem bandeira: as fontes do `egui` não desenham bandeira de forma confiável. No protótipo o
  seletor funciona.
- **Textos:** nomes por idioma, sem mistura. "Commit" vira "Gravar" em português e continua "Commit" em inglês;
  "Run Current" vira "Executar"; "Beautify" vira "Formatar".

## O que o protótipo mostra e ainda não existe

Para a aprovação não confundir desenho com função nova:

- Do M7, só o lugar: abas Consultas e Histórico, seta de chave estrangeira no grid, tabelas na paleta.
- Proposto nesta FEAT (Should do brief): Testar conexão, Enter e Esc no formulário, teclado na lista de conexões,
  "+" na faixa de bancos, botão de ROLLBACK, "Revisar e gravar" na confirmação de fechar.
- Saem da tela até existirem: `+ Row`, `+ Column`, Triggers, Filters, Columns.
