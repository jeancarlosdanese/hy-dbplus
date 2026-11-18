# @file: docs/features/FEAT-015-redesenho-ux/00-brief/brief.md

# FEAT-015: redesenho de UX de todas as telas

## Problema

Pedido do Jean em 17/11/2026: "o sistema está funcional. Agora eu gostaria que você atuasse como um Design e UX
experiente e criasse protótipos para melhorar todas as telas do sistema."

O app faz o que o MVP pede (M0 a M6), mas o acabamento cresceu tela a tela, sem sistema visual. O inventário de
17/11 (leitura de todo `crates/ui-egui/src/`) achou:

- **Idioma misturado na mesma barra:** "Delete" ao lado de "Descartar", "rows" ao lado de "linhas", "Run Current"
  ao lado de "Cancelar". O título do diálogo de gravação sai "1 updates".
- **Sem hierarquia:** tudo é texto do mesmo peso. "Queries" e "History" parecem abas e não clicam. Não há ação
  principal marcada em tela nenhuma.
- **Sem família de ícones:** caracteres soltos e emoji (o barril da faixa de bancos, a chave, o "＋" de largura
  dupla).
- **Mesmo estado, textos diferentes:** seis formas de dizer "carregando", três de "vazio", três de dispensar um
  aviso. Erro do editor sai cinza.
- **Diálogos sem padrão:** três modais e três janelas soltas; ação principal ora à esquerda, ora à direita; ação
  destrutiva sem cor.
- **Cores que se confundem:** célula alterada e linha selecionada usam a mesma cor, então a alteração some numa
  linha selecionada. A etiqueta Dev é igual ao destaque; a Prod é igual ao erro. "Transação aberta" usa vermelho
  de erro para o que é um aviso.
- **Sem escala:** espaçamentos avulsos (4, 6, 8, 10, 12), três tamanhos de fonte definidos, a coluna "#" com 48 num
  grid e 44 no outro.
- **Botões mortos:** cinco botões desabilitados ("em breve", "não disponível nesta versão") ocupam o rodapé.
- **Tela de conexões:** clicar na linha não faz nada, não há "Testar conexão", Enter e Esc não agem no formulário,
  não há atalho nenhum.
- **Atalho com ⌘ fixo:** o rótulo mostra ⌘ também no Linux e no Windows, onde a tecla é Ctrl.

## Objetivo

1. **KR1:** todas as telas e diálogos seguem um sistema visual único (cores, tipos, espaçamento, ícones, botões,
   estados), declarado em `theme.rs`; nenhuma cor nem espaçamento solto fora dele.
2. **KR2:** todo texto da interface existe em português do Brasil e em inglês, sem mistura dentro de um idioma.
3. **KR3:** cada situação (carregando, vazio, erro, só leitura, pendente) tem um texto e um desenho só.
4. **KR4:** o que o app faz hoje continua funcionando: a suíte atual passa, com os rótulos novos.

## Escopo

### Must

- [ ] Sistema visual: paleta nos dois temas, escala de tipo e de espaçamento, raios, ícones, quatro tipos de botão.
- [ ] Redesenho das telas: conexões, formulário, workspace, faixa de bancos, árvore, abas, Dados, painel de
      registro, Estrutura, Índices, DDL, gravação, editor SQL, autocomplete, resultados, paleta, confirmações.
- [ ] Textos em pt-BR e en, com o catálogo num lugar só.
- [ ] Idioma: segue o do sistema (inglês quando não for português) e pode ser trocado no seletor PT | EN da barra
      superior e da tela de conexões; a escolha fica guardada.
- [ ] Alteração pendente com cor própria, distinta da seleção; ambiente sempre com etiqueta de texto.
- [ ] Diálogos todos modais, com cancelar à esquerda da ação principal e as duas à direita.
- [ ] Rótulo de atalho conforme a plataforma (⌘ no macOS, Ctrl nas outras).

### Should

- [ ] Tela de conexões pelo teclado: setas, Enter conecta, linha inteira clicável.
- [ ] Formulário: "Testar conexão", Enter salva, Esc cancela, erro junto do campo.
- [ ] Botão "+" na faixa de bancos; oferta de ROLLBACK quando a transação aborta; "Revisar e gravar" na
      confirmação de fechar.

### Could

- [ ] Seletor de tema dentro do app (hoje o tema segue o sistema).

### Won't

- Filtros, navegação por chave estrangeira, histórico e busca de tabelas na paleta: são do M7. O protótipo mostra
  onde eles entram (abas Consultas e Histórico, seta na coluna de chave estrangeira, tabelas na paleta), mas esta
  FEAT não os implementa.
- `+ Row`, `+ Column`, Triggers, Columns: fora do MVP. Os botões desabilitados saem da tela até existirem.
- Mudança no engine, no driver ou no catálogo, salvo o teste de conexão.

## Reconhecimento do código

| O que já existe | Caminho | Decisão |
|---|---|---|
| Tokens de cor nos dois temas e `apply` | `crates/ui-egui/src/theme.rs` | estender: paleta nova, cores de significado, escala de espaço e de tipo |
| Cores de ambiente e de erro fora dos tokens | `crates/ui-egui/src/theme.rs` (`tag_color`, `error_color`) | estender: entram nos tokens, com valores que não colidem |
| Cabeçalho e célula de grid compartilhados | `crates/ui-egui/src/grid.rs` | estender: o grid da aba Dados passa a usar os mesmos |
| Três modais e três janelas soltas | `crates/ui-egui/src/commit.rs`, `crates/ui-egui/src/structure.rs`, `crates/ui-egui/src/rail.rs`, `crates/ui-egui/src/connections.rs`, `crates/ui-egui/src/palette.rs` | novo: um componente de diálogo só |
| Textos literais espalhados em 15 arquivos | `crates/ui-egui/src/` | novo: catálogo de textos pt-BR e en |
| Glifos Unicode como ícone | `crates/ui-egui/src/tree.rs`, `crates/ui-egui/src/rail.rs`, `crates/ui-egui/src/tabs.rs`, `crates/ui-egui/src/data.rs` | novo: fonte de ícones embutida |
| Fontes padrão do egui | feature `default_fonts` do eframe | decidir no plano: embutir Inter e uma fonte de largura fixa |
| Testes de UI que buscam por rótulo | testes em `crates/ui-egui/src/tree.rs` e `crates/ui-egui/src/app.rs` (36 testes) | estender: os rótulos mudam junto, o teste fixa o idioma |
| Teste de conexão | `crates/engine/` (conectar já devolve versão ou erro) | reaproveitar: testar é conectar e fechar |

Duplicidade ou conflito encontrado: a seção Processo do `CLAUDE.md` diz que o projeto não tem pasta de protótipos.
Esta FEAT usa a pasta `20-prototipos/` dela mesma, em HTML, fora do workspace Rust e do build. A regra "Rust only"
vale para o produto e o build e não é tocada.

## Restrições

- Tudo precisa caber no `egui` 0.36: sem desfoque, sem animação além do indicador de carregamento, sombra só em
  janela. O protótipo foi desenhado dentro disso.
- Fonte e ícone novos são dependência ou arquivo embutido: licença conferida contra o `deny.toml` e peso do binário
  medido antes de decidir.
- Never panic, layering e async fora da UI continuam valendo. O catálogo de textos não entra abaixo da UI.
- A suíte de testes de UI depende de rótulos: a troca de textos é feita junto com os testes, tela a tela.

## Decisões do mantenedor (17/11/2026)

- Direção visual: manter a estrutura do TablePlus e dar identidade própria no acabamento, com o azul-petróleo da
  Hyberica como destaque, família de ícones e escala de tipografia e espaçamento ("Recomendado").
- Idiomas: "Português Brasil e Inglês (i18n)".
- Escolha do idioma: "seguir o idioma do sistema, mas acho bom dar escolha, um ícone us | pt" na barra superior,
  ao lado da busca ⌘K.

## Decisões abertas

Nenhuma.

## Referências

- Protótipo: `../20-prototipos/prototipo.html`.
- `docs/BRIEF.md` seção 3 (telas de referência do TablePlus) e marcos M7 e M8.
- Logomarca: `packaging/icon.svg` (azul-petróleo `#1C4F5E` a `#07252E`).
