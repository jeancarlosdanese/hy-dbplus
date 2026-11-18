# @file: docs/features/FEAT-015-redesenho-ux/50-dev/impl-notes.md

# Notas de implementação: FEAT-015

> Protótipo aprovado pelo Jean em 17/11/2026 ("pode seguir"), como desenhado: ambiente em etiqueta e faixa fina.
> Aceite e plano aprovados no mesmo dia ("pode seguir"). Fases A (e461e98), B (1406e00), C (43b7515) e D
> (6c01cc4) e E (e11c7f3) commitadas.

## Decisões de implementação

- **Textos:** catálogo próprio em `crates/ui-egui/src/i18n.rs`, sem crate de tradução. Um tipo `Lang` (pt-BR, en)
  e uma função por texto; texto com número recebe o número e resolve o plural. O compilador garante que os dois
  idiomas têm todos os textos. Motivo: são cerca de 250 textos, dois idiomas, e uma crate de i18n traria formato
  de arquivo e carga em tempo de execução que o app não precisa.
- **Idioma do sistema:** crate `sys-locale` (MIT ou Apache-2.0, sem dependências no macOS além das do sistema).
  Dependência nova, justificada: app aberto pelo Finder não recebe `LANG`, então variável de ambiente não serve.
  Conferir no `cargo deny` antes de fixar.
- **Preferência:** arquivo `preferences.toml` ao lado do de conexões, só com o idioma. Nunca guarda segredo. Arquivo
  ausente, ilegível ou com valor desconhecido vale como "seguir o sistema".
- **Fontes:** Inter (interface) e JetBrains Mono (dados e SQL), embutidas com `include_bytes!`, licença OFL-1.1.
  Medir o binário antes e depois; teto aceito de 1,5 MB a mais. Se passar, fica só a Inter.
- **Ícones:** fonte Phosphor (MIT) embutida, com uma tabela própria só dos ícones usados. Usar a crate
  `egui-phosphor` só se ela aceitar o `egui` 0.36 sem duplicar versão; senão, o arquivo da fonte direto.
- **Componentes:** módulo `crates/ui-egui/src/widgets.rs` com botão (principal, comum, discreto, destrutivo),
  controle segmentado, etiqueta de ambiente, faixa de aviso e de erro, estado vazio, carregando e o diálogo modal.
  As telas param de montar isso à mão.
- **Testes de UI:** o harness fixa o idioma em português, para os testes não dependerem da máquina. Rótulo novo
  entra no teste na mesma fase em que a tela muda.

## Princípios

- O protótipo aprovado é o contrato visual; desvio dele é registrado em "Desvios do plano" (17/11/2026, Jean).
- Uma tela por vez: cada fase deixa o app inteiro funcionando, mesmo com telas ainda no desenho antigo.
- Nome de objeto do banco, tipo, SQL e erro do servidor nunca são traduzidos.

## Plano

Cada fase fecha com teste, notas e o "pode commitar" do Jean. Não há passo que só o mantenedor execute, além da
conferência visual no fim de cada fase.

### Fase A: fundação (tema, fontes, ícones, componentes, idioma)

- [x] A.1 `theme.rs`: paleta nova nos dois temas, cores de significado (pendência, aviso, erro, ok), cores de
      ambiente que não colidem, escala de espaçamento e de tipo, raios. Teste: pendência difere de seleção, e
      ambiente difere de destaque e de erro, nos dois temas.
- [x] A.2 Fontes Inter e JetBrains Mono embutidas e registradas; medir o binário.
- [x] A.3 Fonte de ícones e a tabela dos ícones usados.
- [x] A.4 `widgets.rs`: botões, segmentado, etiqueta de ambiente, faixas, estados e diálogo modal, com teste de
      Esc e de clique fora.
- [x] A.5 `i18n.rs`: `Lang`, catálogo com os textos comuns, plural, rótulo de atalho por plataforma. Testes de
      plural e de atalho.
- [x] A.6 Idioma do sistema (`sys-locale`) e `preferences.toml`: ler, gravar, cair no padrão. Testes dos três
      cenários de idioma.
- [x] A.7 Seletor PT | EN na tela de conexões e na barra superior, ligado à preferência.
- [x] A.8 Harness de teste com idioma fixo; suíte verde; portão completo; revisor.

### Fase B: conexões

Objetivo: tela de conexões, formulário, remover, com teclado e Testar conexão (telas 1 a 4). Critérios: AC-008
(diálogos), AC-009 (Enter e erro no campo), AC-010 (Testar conexão), AC-011 (teclado na lista), e os textos dessas
telas nos dois idiomas. Não há passo do mantenedor além da conferência visual.

- [x] B.0 Detalhar os passos da fase (17/11/2026).
- [x] B.1 Engine: `Sessions::test_connection(id, config, senha)` conecta num cliente à parte, lê a versão, fecha e
      emite `Event::TestFinished { connection, result }` com o tempo em ms ou o erro do driver. Não toca a sessão
      aberta, o arquivo de conexões nem o Keychain. Na edição com senha em branco, usa a do Keychain. Testes com o
      driver falso: sucesso com tempo, erro do servidor, sessão aberta intacta, nada gravado, senha fora do erro.
- [x] B.2 Lista (telas 1 e 2): cabeçalho com marca, título, busca com ícone, PT | EN e "Nova conexão" principal;
      linha de 56 px com a barra do ambiente, o selo Pg, nome, etiqueta, estado (conectado, conectando, erro na
      própria linha) e `host:porta / banco` em mono; linha inteira clicável (clique seleciona, duplo clique conecta
      ou abre); editar e remover como ícones só na linha selecionada ou com erro; Conectar ou Abrir à direita,
      principal na selecionada. Rodapé com os atalhos. Teclado: ↑↓ movem a seleção, Enter conecta ou abre, ⌘N
      abre o formulário novo, ⌘E edita a selecionada. Estado vazio da tela 2. Todos os textos pelo catálogo.
- [x] B.3 Formulário (tela 3): `widgets::dialog` de 560 px em duas colunas, rótulo acima do campo, ambiente com a
      etiqueta, dica da senha (nova: em branco usa `trust`; edição: em branco mantém a atual), "Banco inicial".
      Validação por campo: `ConnectionForm::build` passa a devolver o campo e a mensagem, e o erro aparece sob o
      campo, nos dois idiomas. Enter salva quando válido; Esc e clique fora cancelam. "Testar conexão" no rodapé,
      com o resultado ao lado ("Conectou em 84 ms" ou o erro do servidor) e o botão desligado enquanto testa.
- [x] B.4 Remover (tela 4): `widgets::dialog` com o nome no título, o texto do que sai junto e o botão vermelho
      "Remover conexão" com ícone.
- [x] B.5 Ícones com nome acessível: `widgets::icon_button` publica a dica como rótulo de acessibilidade, para o
      leitor de tela e para os testes acharem "Editar" e "Remover".
- [x] B.6 Testes de UI dos cenários AC-008 a AC-011 e rótulos novos nos testes antigos; suíte, portão completo,
      revisor e conferência visual das telas 1 a 4 nos dois temas e idiomas.

### Fase C: workspace

Objetivo: barra superior, faixa de bancos, árvore, abas, paleta e confirmações (telas 5, 6 e 16). Critérios:
AC-008 (diálogos), AC-013 (fechar com a contagem e as três saídas), AC-014 (nenhum botão morto) e os textos dessas
telas nos dois idiomas. Não há passo do mantenedor além da conferência visual.

- [x] C.0 Detalhar os passos da fase (17/11/2026).
- [x] C.1 Barra superior (telas 5 a 16): fundo neutro em vez de pintado pelo ambiente; "← Conexões" discreto,
      etiqueta do ambiente, `conexão / banco` e a versão do PostgreSQL em texto apagado; à direita o campo "Ir para
      banco ou tabela ⌘K" (abre a paleta), PT | EN, e os ícones do painel de registro e de desconectar, com dica e
      nome acessível. Sob a barra, a faixa de 3 px na cor do ambiente. Avisos ("Abrindo banco…", erro do banco, aviso
      do app) em `widgets::banner` e `widgets::loading`.
- [x] C.2 Faixa de bancos: item com o ícone de banco e o nome, o ativo destacado, × para fechar (com a mesma
      confirmação de hoje) e um "+" no fim que abre a paleta.
- [x] C.3 Árvore: busca com ícone ("Buscar tabela ou função"), grupos "Tabelas" e "Funções" com a contagem e seta de
      abrir; tabela, view e tabela externa com ícone Phosphor em vez de ▦ ◫ ⇄; coluna com o ícone de chave na chave
      primária e o tipo em texto apagado; rodapé com o seletor de schema e o ícone de recarregar. Os rótulos
      "Items", "Queries" e "History" saem: Queries e History são do M7 e hoje são texto sem ação (AC-014).
- [x] C.4 Abas: aba com ícone (tabela ou SQL), nome, ponto âmbar de pendência e × ; a ativa colada ao conteúdo; "+"
      com a dica "Nova consulta SQL ⌘T". Estado vazio da área central (tela 5): "Nada aberto", com ⌘K e ⌘T.
- [x] C.5 Paleta (tela 6): `widgets::dialog` sem rodapé, fundo escurecido, busca com ícone e "esc", grupo "Bancos",
      item com ícone e "↩ abrir" no selecionado; a rolagem e as setas da fase B continuam.
- [x] C.6 Fechar com pendências (tela 16): `widgets::dialog_full` com o nome no título, a contagem ("Há 2
      alterações e 1 exclusão que ainda não foram gravadas"), e as três saídas: "Revisar e gravar" (fecha a
      confirmação e abre a gravação da aba com pendência), Cancelar e "Descartar e fechar" em vermelho. Aba de SQL
      com texto, banco e conexão mantêm os textos próprios.
- [x] C.7 Testes dos rótulos novos nos testes antigos da árvore, das abas e da faixa; testes de AC-013 e AC-014 nas
      telas da fase; suíte, portão completo, revisor e conferência visual.

### Fase D: dados

Objetivo: aba Dados, edição, painel de registro, gravação, Estrutura, Índices e DDL (telas 7 a 12).

- [x] D.0 Detalhar os passos da fase (17/11/2026).
- [x] D.1 Barra da aba de tabela (telas 7 a 12), em cima do conteúdo: segmentado Dados | Estrutura | Índices
  (`widgets::segmented`), `schema.tabela` em destaque com "1.284 linhas · 312 kB" (milhar pelo idioma),
  comentário da tabela na dica, aviso "só leitura (sem chave primária)" como etiqueta, e o recarregar por ícone
  (⌘R, travado com pendência; refaz a página atual pelo `load_data_page` que já existe). Em Estrutura, o botão
  "Ver DDL" na mesma barra.
- [x] D.2 Rodapé da aba Dados (tela 7 e 8): sem pendência, a dica "Duplo clique edita a célula"; com pendência, o
  chip âmbar "2 alterações, 1 exclusão", "Descartar ⌘Z" (fantasma) e "Revisar e gravar ⌘S" (principal). À direita,
  "Linhas por página" 100/500/1000, "1–100 de 1.284" e ‹ › por ícone, travados com pendência. Saem "+ Row",
  "Filters" e "Columns" (AC-014). "Apagar linhas" (⌫) fica como botão fantasma só com linha selecionada em tabela
  editável, para o mouse não perder o caminho.
- [x] D.3 Grade (telas 7 e 8): cabeçalho com o ícone de chave na chave primária, o tipo pequeno embaixo do nome e
  a seta de ordenação por ícone; número alinhado à direita pelo tipo do catálogo; NULL em itálico apagado; célula
  pendente com o fundo e o texto âmbar, que vencem a seleção (AC-006, regra numa função pura com teste); linha
  marcada para apagar riscada, no vermelho do tema; campo de edição com a borda do destaque.
- [x] D.4 Painel de registro (tela 8): cabeçalho "Linha 3" (ou "3 linhas") com o segmentado Campos | JSON; cada
  campo com nome, tipo apagado à direita, etiqueta "alterado" e o valor pendente em âmbar; chave primária só
  leitura; "Definir NULL" e "Reverter campo"; "Múltiplos valores", "marcada para apagar", "mostrar tudo" e
  "Copiar" no catálogo. Na aba SQL, o JSON da linha com "Copiar".
- [x] D.5 Diálogo de gravação (tela 9) por `widgets::dialog_full`: título "Gravar 2 alterações e 1 exclusão em
  pedidos" com plural nos dois idiomas (AC-007); subtítulo com a etiqueta do ambiente e "Roda numa transação: ou
  grava tudo, ou não grava nada."; o SQL em bloco mono com as cores de sintaxe do tema e o comando que falhou em
  vermelho; erro do servidor em `widgets::banner`; rodapé "Copiar SQL" à esquerda, Cancelar e "Gravar no banco
  ⌘↩". Durante a gravação, Esc, clique fora e os botões não fecham (AC-008); o `dialog_full` ganha esse modo.
- [x] D.6 Estrutura e Índices (telas 10 e 11): seções "Colunas" (#, Coluna, Tipo, Aceita nulo, Padrão,
  Comentário, com o ícone de chave) e "Restrições" (Nome, Tipo por extenso, Colunas, Definição); "Índices" (Nome,
  Método, Único, Colunas, Condição, Comentário); "sim/não" e tipos de restrição no catálogo; rodapé "6 colunas ·
  3 restrições" e "3 índices"; carregando, erro com "Tentar de novo" e vazio pelos componentes. Saem "+ Column" e
  "Triggers" (AC-014).
- [x] D.7 DDL (tela 12) por `widgets::dialog_full`: título "DDL: public.pedidos", subtítulo "Montado a partir do
  catálogo. Só leitura.", código mono com as cores de sintaxe, Fechar e "Copiar" (principal).
- [x] D.8 Textos: `LOCK_HINT`, `READ_ONLY_HINT` e os literais de data, record, commit, structure e index passam
  para o catálogo nos dois idiomas. Testes: rótulos novos nos testes antigos; AC-006 (pendência vence a seleção),
  AC-007 (título com plural nos dois idiomas), AC-008 (gravação em curso não fecha por Esc nem por clique fora),
  AC-014 (nenhum dos cinco botões mortos), ⌘R recarrega e fica travado com pendência, tela de dados em inglês.

### Fase E: editor SQL

Objetivo: barra do editor, autocomplete, resultados, erro e ROLLBACK (telas 13 a 15).

- [x] E.0 Detalhar os passos da fase (18/11/2026).
- [x] E.1 Barra do editor (tela 13): Executar (principal, ⌘↩), Executar tudo (⌘⇧↩), Cancelar (só ligado durante a
      execução) e Formatar (⌘I), com o atalho no botão; durante a execução, spinner e "Executando · 3,4 s"; à
      direita a etiqueta âmbar "Transação aberta" com a dica e "Limite" com "500 linhas" ou "Sem limite".
- [x] E.2 Editor: cores de sintaxe do tema (`Colors::of`), números de linha no fundo do painel com borda, fundo de
      campo, statement sob o cursor com fundo e traço de destaque à esquerda; rodapé com "Linha 6, coluna 43" e
      "Salva sozinha ao sair do editor" (o app já salva a consulta quando o editor perde o foco).
- [x] E.3 Autocomplete (tela 14): ícone por tipo, nome em fonte fixa, tipo e detalhe à direita traduzidos, item
      escolhido em destaque suave e rodapé com as teclas (↑↓ navegar, ↩ ou tab aceitar, esc fechar); "carregando
      colunas…" no catálogo.
- [x] E.4 Resultados: sub-abas "Resultado N" com ícone de ok ou de erro; à direita "5 linhas · 12 ms", "truncado
      em 500" ou "falhou em 3 ms"; "OK, 3 linhas afetadas" traduzido; sem resultado, estado vazio com os atalhos
      da plataforma; executando, "Executando a consulta 2 de 3" (o `total` do `SqlStarted`, hoje ignorado).
- [x] E.5 Erro e transação (tela 15): erro em faixa vermelha com o comando em bloco de SQL; transação abortada
      ganha a faixa âmbar "A transação foi abortada. Execute ROLLBACK antes de continuar." com "Executar
      ROLLBACK", que roda `ROLLBACK` no banco (AC-012); transação aberta é só aviso âmbar, nunca vermelho.
- [x] E.6 Avisos do editor no catálogo: o `notice` da aba de SQL deixa de ser texto em português e vira um tipo
      traduzido pelo idioma ativo (cancelada, outra execução em curso, nada para executar, statement incompleto,
      gravação em curso, COMMIT que virou ROLLBACK, falha ao salvar, erro do SQL gerado); aparece em faixa de aviso
      sob a barra, com fechar.
- [x] E.7 Testes e revisão: rótulos novos na suíte; testes novos para o ROLLBACK (só com a transação abortada, e
      roda `ROLLBACK`), a etiqueta âmbar da transação aberta, a barra durante a execução, o editor em inglês e o
      rodapé do autocomplete; portão `--full`; revisor.

### Fase F: fechamento

Objetivo: varredura de textos, cores e espaçamentos soltos; conferência nos dois temas e idiomas; `docs/BRIEF.md`
e a linha "Padrão visual e protótipos" do `CLAUDE.md` atualizados.

- [ ] F.0 Detalhar os passos da fase.

## Desvios do plano

- **B.3, testar na edição:** com a senha em branco, o teste usa a do Keychain; erro do chaveiro aparece como
  resultado do teste. O resultado só vale para o pedido em curso do formulário aberto (etiqueta por pedido).
- **B.3, onde o erro aparece:** o campo errado ganha a borda vermelha e a mensagem sob ele; a validação parou de
  devolver texto e devolve o campo (`Field`), e a mensagem vem do catálogo.
- **B.2 e B.3, Enter:** só vale como "conectar" ou "salvar" com o foco num campo de texto (ou em nada); num botão
  focado, o Enter é do botão. Achado do revisor: Enter no Cancelar salvava a edição.
- **B.2, duplo clique:** a linha inteira responde; a seleção de texto dos rótulos foi desligada dentro dela, senão
  o rótulo pegava o clique.
- **Conferência do Jean na fase B (17/11/2026):** a lista da árvore passava por baixo da barra "Schema" sem
  barra de rolagem visível (a do egui é flutuante e só aparece com o ponteiro em cima). A barra passou a ficar
  sempre visível e com espaço próprio em todo o app (`theme::scroll_style`). Na mesma captura, os símbolos ▦ ◫ ▸ ◂
  ⇄ ▾ e o 🛢 da faixa saíam como quadrados; não era regressão da fase A (a família proporcional padrão do egui
  também não os tem). A Hack, que já vem no binário, entrou por último na família proporcional, e a faixa usa o
  ícone de banco do Phosphor.
  Na segunda captura ficou claro que a lista era a da paleta "Escolher banco" (⌘K): com 60 bancos ela passava do
  fim da janela. A lista ganhou altura máxima com rolagem, e a seleção pelas setas rola junto (as teclas passaram
  para antes da lista). O redesenho da paleta segue na fase C. Teste com 60 bancos, provado contra a versão antiga.
- **C.1, campo da paleta:** o protótipo diz "Ir para banco ou tabela", mas a paleta só acha bancos (tabelas são
  do M7). O texto ficou "Ir para banco", para não prometer o que não existe (AC-014).
- **C.3, Items, Queries e History:** saíram do painel esquerdo; Queries e History são do M7 e eram texto sem ação.
  Volta quando o M7 existir.
- **C.3, ordem dos grupos:** Tabelas antes de Funções, como no protótipo. Tabela externa ganhou o ícone de setas
  do Phosphor (novo no recorte).
- **C.4, título da aba SQL:** continua "SQL Query N" em inglês nos dois idiomas; o título é gravado com a aba e
  muda na fase E, com o editor.
- **C.6, "Revisar e gravar":** fecha a confirmação, ativa a primeira aba com pendência do alvo (aba, banco ou
  conexão), põe na sub-aba Data e abre a caixa de gravação. Só aparece com pendência de tabela (não para texto de
  SQL nem para célula aberta sem outra pendência). Na conexão, o botão vermelho diz "Descartar e desconectar".
- **Aba Structure:** o 🔑 da lista de colunas da Structure fica para a fase D.
- **D.1, recarregar só em Dados:** o protótipo mostra o ícone também em Estrutura e Índices; lá o catálogo já se
  recarrega pela árvore, e o ícone fica só na sub-aba Dados.
- **D.1, comentário e tamanhos:** o comentário da tabela e os tamanhos de dados e de índices passaram para a dica do
  nome e do resumo; a barra mostra só "linhas · tamanho total". A dica "grave ou descarte antes" também saiu da
  linha de cima e ficou só na dica dos controles travados.
- **D.2, "Apagar linhas":** continua como botão fantasma, só com linha selecionada em tabela que edita.
- **D.5, atalho no botão:** o `DialogAction` ganhou o atalho à direita (`⌘↩`) e o `dialog_full` ganhou o modo
  ocupado. O nome acessível dos botões com atalho é só o rótulo, para os testes não dependerem da plataforma.
- **D.5, comando que falhou:** em vez de pintar o comando dentro do bloco de SQL, ele aparece em vermelho sob a
  faixa do erro, como no protótipo do editor.
- **D.7, Copiar no DDL:** é a ação principal e não fecha o diálogo; Fechar e Esc fecham.
- **Aviso de página descartada** (`DROPPED_HINT`, `app.rs`): ainda em português; vem de dentro do estado, sem o
  idioma, e entra na varredura de textos da fase F.
- **Editor SQL:** o realce do editor usava as cores antigas até a fase E; agora usa as cores de sintaxe do tema,
  como a gravação e o DDL.
- **E.5, faixa do ROLLBACK:** como no protótipo, fica sob o erro quando a sub-aba que falhou está ativa; com outra
  sub-aba ativa, sobe para o alto do painel, para não sumir enquanto a transação estiver abortada. A etiqueta
  "Transação aberta" continua na barra com a transação abortada (ela segue aberta no servidor), como na tela 15.
- **E.3, detalhe da sugestão:** o motor de autocomplete (`crates/sql`) devolvia o detalhe pronto em português
  ("tabela · public", "integer · chave"). Agora devolve só dados (tipo, schema, assinatura) e marca a chave e o
  tipo de tabela; a UI monta o texto no idioma, sem repetir o tipo.
- **E.6, título da aba SQL:** "SQL Query N" era um campo gravado em português e inglês misturados; virou
  "Consulta N" e "Query N", calculado na hora pelo idioma (`QueryTab::title`). Entrou nesta fase porque é o texto
  do editor que aparece na aba e nas confirmações.
- **E.6, avisos:** os avisos viram faixa âmbar sob a barra (falha ao salvar e erro do SQL gerado em vermelho), com
  Fechar. O texto do "gravação em curso" deixou de dizer "Commit" em português.
- **E.4, coluna "#":** a largura passou a ser uma só (`grid::NUMBER_WIDTH`, 48) no grid de dados e no de
  resultados; o brief apontava 48 num e 44 no outro.
- **B.4, texto do remover:** o protótipo diz que as consultas salvas da conexão ficam no disco, mas desde a FEAT-011
  remover a conexão apaga também as consultas dela. O texto do diálogo segue o comportamento: "A senha guardada no
  Keychain e as consultas salvas desta conexão também são apagadas."

- **A.2 e A.3, fontes:** entram Inter Regular e SemiBold (família `strong`) e JetBrains Mono Regular, sem itálico.
  Os ícones vêm da crate `egui-phosphor` 0.14 (compatível com o `egui` 0.36), com a feature `subset`: a fonte é
  cortada em tempo de compilação e só os 30 ícones da tabela `fonts::icon` vão ao binário. Binário de release:
  11.270.096 → 12.413.536 bytes (+1,14 MB, dentro do teto de 1,5 MB).
- **A.6, onde mora a preferência:** `preferences.toml` é lido e gravado pelo engine (`PreferenceStore` e o tipo
  `Language`), não pela UI, como o `connections.toml`. A UI usa `Language` direto; o `Lang` do plano virou esse
  tipo. A gravação atômica (diretório 0700, arquivo 0600, `.tmp` renomeado) saiu do `ConnectionStore::save` para
  `save_toml`, usada pelos dois.
- **A.4, texto forte sem as fontes:** `fonts::strong(ctx)` cai na família padrão quando as fontes do app não
  estão instaladas (harness de teste, contexto novo). Família nomeada inexistente derruba o `egui`.
- **Código ainda sem uso:** ícones, componentes e a maior parte do catálogo só entram nas telas das fases B a E.
  Os três módulos levam `#[allow(dead_code)]` com o comentário, e a fase F tira.
- **`theme::error_color(ctx)`** passou a devolver o `Tokens::danger` da variante na tela (achado do revisor: o
  vermelho fixo caía abaixo do contraste legível no escuro). As 18 chamadas ganharam o contexto; cada tela troca
  por `tokens.danger` na sua fase.
- **Textos traduzidos já na fase A** (achado do revisor: o seletor não mudava nada visível): cabeçalho das
  conexões (título, Nova, Buscar, lista vazia, busca sem resultado) e barra superior (Conexões, Desconectar,
  Banco com o atalho por plataforma, Painel, Escolher banco). O resto das telas segue em português até a sua fase.
- **Aviso do app no workspace** (achado do revisor): `App::notice` só aparecia na lista de conexões; agora aparece
  numa faixa de erro sob a barra superior, a menos que seja o mesmo erro já mostrado na faixa da conexão (aí o OK
  dela limpa os dois).

## Revisão

| Passada | Base | Achados | Destino |
|---|---|---|---|
| 1ª | `git diff main` e arquivos novos | 1 médio (falha ao gravar o idioma invisível no workspace); 3 baixos (vermelho de erro no escuro, seletor sem efeito visível, comentário de `fonts::strong`) | todos corrigidos; testes novos para o aviso e para a cor de erro |
| 2ª | delta das correções | 3 baixos (mesmo erro em duas faixas, dica da lista vazia fixa em português, "Escolher banco (⌘K)" fixo) | todos corrigidos, com teste da duplicação provado contra o código sem o filtro; sem nova passada, como a skill manda para baixo do delta; a passada da branch inteira fica para o fechamento |
| B, 1ª | `git diff HEAD` (fase B) | 2 médios (Enter tomado antes dos widgets: Enter no Cancelar salvava e derrubava a sessão; botões cobriam o erro do teste e o da linha); 1 baixo (sem teste da senha do Keychain no teste da edição) | todos corrigidos; testes do Enter e do rodapé provados contra o código antigo; testes do Keychain (senha guardada e chaveiro falhando) |
| B, 2ª | delta das correções | 1 baixo (nome e endereço longos passavam sob os botões) | corrigido (largura limitada e truncamento), teste provado contra o código antigo; sem nova passada |
| C, 1ª | `git diff HEAD` (fase C) e os ajustes do 1406e00 sem passada (barra de rolagem, reserva Hack, ícone da faixa, rolagem da paleta) | 2 baixos (trocar o idioma fechava as tabelas abertas na árvore, porque o id do grupo vinha do rótulo traduzido; textos substituídos ficaram no catálogo, escondidos pelo `allow(dead_code)`) | ambos corrigidos: id fixo nos grupos, com teste provado contra o código sem ele; cinco textos mortos removidos e o teste da reserva Hack ajustado aos símbolos em uso (▲▼ do grid) |
| C, 2ª | delta das correções | nenhum | fase liberada |
| D, 1ª | `git diff HEAD` (fase D) | 2 médios (⌘↩ gravava com a paleta ou a confirmação de fechar por cima; ⌘R com célula aberta recarregava e perdia o digitado); 5 baixos (recarregar não pedia os tamanhos; array de `numeric` alinhado como número; `ConstraintKind::label` sem chamador; `lock_detail` sem uso; montagem das partes repetida em três funções) | todos corrigidos. Ao testar o primeiro, apareceu outro: com a paleta sobre a gravação, um Esc fechava as duas. ⌘K, ⌘T e ⌘W agora não agem com a gravação ou o DDL abertos. Testes provados contra o código sem cada conserto |
| D, 2ª | delta das correções | 2 baixos (a guarda dos atalhos olhava o banco ativo mesmo com outra conexão em foco e travava ⌘K, ⌘T e ⌘W sem diálogo na tela; comentário de `retry_data` fora do lugar) | ambos corrigidos, o primeiro com teste provado contra o código sem o filtro; sem nova passada, como a skill manda para baixo do delta |
| E, 1ª | `git diff HEAD` (fase E) | 2 médios (detalhe do autocomplete em português no app em inglês e com o tipo repetido; o aviso de statement incompleto era apagado pelo `SqlStarted` e nunca aparecia, defeito anterior à fase); 5 baixos (traço do statement com a faixa do texto antigo ao digitar; `theme::error_color` sem uso; nomes antigos em comentários; margens e medidas soltas; Status da FEAT desatualizado e faixa do ROLLBACK acima do erro) | todos corrigidos; testes do detalhe traduzido e do aviso de incompleto, este provado contra o código sem o conserto |
| E, 2ª | delta das correções | 1 baixo (a escolha do ícone por tipo de tabela ficou copiada na árvore e no autocomplete) | corrigido: `fonts::icon::table_kind` usado pelos dois; sem nova passada, como a skill manda para baixo do delta |

## Testes

**Conferido:** protótipo conferido por captura de tela em 17/11/2026 (telas 3, 7, 8, 13, 15 e 17, nos dois temas
e idiomas).

Fase A (17/11/2026), por teste:
- Tema: pendência difere de seleção, hover e listra; ambientes distintos entre si, do destaque e do erro, com
  contraste ≥ 3 sob texto branco; contraste WCAG do texto (≥ 7), do secundário e do erro (≥ 4,5) sobre painel,
  cartão e campo, e do texto sobre botão principal, pendência e seleção, nos dois temas.
- Fontes: ordem das famílias, glifo dos ícones, de "ç", "ã" e "⌘" na Inter com reservas, e "λ" na mono; texto
  forte sem fontes instaladas cai na padrão; o corte dos ícones fica abaixo de 60 KB (asserção em compilação).
- Idioma: locale do sistema (pt-BR, pt_PT, `PT_br.UTF-8` viram português; es, fr, vazio e ausente viram inglês);
  preferência vence o sistema; arquivo ausente, TOML inválido, valor desconhecido, binário e diretório no lugar do
  arquivo seguem o sistema; escolha persiste; falha ao gravar devolve erro e a escolha vale na sessão, com aviso;
  arquivo 0600.
- Textos: plural e milhar nos dois idiomas; atalho `⌘K` no macOS e `Ctrl+K` fora dele.
- Componentes: diálogo fecha por Esc, por clique fora e pelos botões, e não fecha com clique dentro; segmentado só
  avisa na troca; faixa devolve o clique da ação.
- Seletor PT | EN: na tela de conexões e na barra superior troca o idioma e grava a escolha; o harness abre em
  português. Falha ao gravar a partir do workspace aparece na tela e fecha; erro repetido aparece uma vez só (os
  dois testes falham sem o conserto).
- Cor de erro das telas segue o tema e passa de 4,5 de contraste nos dois.

Fase B (17/11/2026), por teste:
- Engine: teste de conexão devolve versão e tempo sem abrir sessão; com erro, não expõe a senha e não derruba a
  sessão aberta.
- Lista: estado vazio com o botão que abre o formulário; ↓↑ movem a seleção; ⌘E edita, ⌘N abre o novo, Esc fecha;
  Enter e duplo clique conectam e abrem o workspace; clique seleciona e mostra Editar e Remover; tela em inglês.
- Formulário: validação por campo nos dois idiomas; Enter com porta "54321x" não salva e mostra "Use um número de
  1 a 65535."; com a porta certa salva; clique fora fecha sem salvar.
- Testar conexão: mostra "Conectou em … ms" e a versão, ou o erro do servidor; nada vai ao arquivo nem ao chaveiro,
  nenhuma sessão abre e a senha não aparece; resposta de outro pedido é ignorada (falha sem a etiqueta).
- Remover: título com o nome, Esc cancela, o botão vermelho remove.
- Enter num botão focado é do botão (Cancelar cancela sem salvar; Remover abre a confirmação e não conecta).
- Erro longo do teste e da linha, nome e endereço longos: nenhum texto passa sob os botões.
- Testar na edição com a senha em branco usa a do Keychain; com o chaveiro falhando, mostra o erro e não conecta.

Fase C (17/11/2026), por teste:
- Os 12 testes antigos do workspace, da árvore, das abas e da faixa passaram para os rótulos novos e seguem verdes.
- Fechar aba com 1 alteração e 1 exclusão: título com o nome, texto com a contagem, Esc cancela, "Revisar e
  gravar" abre a gravação sem fechar, "Descartar e fechar" fecha (AC-013). Plural e milhar do texto nos dois
  idiomas.
- Barra superior com `conexão / banco`, versão e etiqueta; nenhum "Items", "Queries" ou "History" (AC-014); o
  ícone do painel abre o painel; o "+" da faixa abre a paleta; a paleta fecha por Esc e por clique fora (AC-008).
- Workspace em inglês.
- Trocar PT | EN com uma tabela aberta na árvore mantém as colunas à mostra (falha sem o id fixo do grupo).

Fase D (18/11/2026), por teste:
- Os testes antigos da grade, da edição, da gravação, da Estrutura, dos Índices e do DDL passaram para os rótulos
  novos e seguem verdes.
- Pendência e exclusão pintam a célula por cima da cor de seleção, nos dois temas (AC-006); números alinham à
  direita pelo tipo.
- Título da gravação com contagem e plural nos dois idiomas (AC-007); gravação em curso não fecha por Esc, clique
  fora nem Cancelar, e parada fecha pelos dois sem descartar (AC-008).
- Nenhum "+ Row", "Filters", "Columns", "+ Column", "Triggers" nem "em breve" em Dados, Estrutura e Índices
  (AC-014).
- ⌘R e o ícone recarregam a mesma página; com pendência, nenhum dos dois recarrega.
- Aba de tabela e gravação em inglês.
- Com a gravação aberta, ⌘K, ⌘T e ⌘W não abrem nada e só ⌘↩ grava; ⌘R não recarrega com célula aberta e pede
  de novo os tamanhos (os dois testes falham sem o conserto).
- Portão completo verde.
- Portão completo verde e `cargo deny check advisories` ok.

Fase E (18/11/2026), por teste:
- Os testes antigos do editor, do autocomplete, do SQL gerado e das consultas salvas passaram para os rótulos e para
  o aviso tipado, e seguem verdes.
- A faixa do ROLLBACK só aparece com a transação abortada, e o botão roda `ROLLBACK` e fecha a transação (AC-012;
  falha sem a condição e sem a chamada).
- Durante a execução só Cancelar fica ligado; tempo e "Executando a consulta 1 de 1" aparecem; o aviso de cancelada
  fecha pelo Fechar da faixa.
- Editor em inglês: barra, rodapé, estado vazio, resultados, linhas afetadas, "failed in" e o aviso, que troca de
  idioma com ele na tela.
- Rodapé do autocomplete com as teclas; detalhe da sugestão no idioma, sem repetir o tipo.
- O aviso de statement incompleto aparece depois de rodar e some na execução seguinte (falha sem o conserto).

**Não conferido:**
- Nitidez da Inter em 12 e 13 px e o visual geral no app: a captura de tela não tem permissão nesta sessão. Fica
  para a conferência do Jean.
- Idioma do sistema real em outra língua (só por teste da função de locale).
- Linux e Windows (só o rótulo de atalho por teste).
- Fase B no app: visual das telas 1 a 4 nos dois temas e idiomas, foco do teclado com o campo de busca ativo, e o
  "Testar conexão" contra um servidor real. Conferência do Jean.
- Fase C no app: visual das telas 5, 6 e 16 nos dois temas e idiomas; a faixa de ambiente com cada tag.
- Fase D no app: visual das telas 7 a 12 nos dois temas e idiomas; ⌘↩ gravando contra o banco local.
- Fase E no app: visual das telas 13 a 15 nos dois temas e idiomas (cor âmbar da etiqueta, traço do statement
  atual, popup); ROLLBACK contra o banco local.

## Segurança (fechamento)

| Rota ou tela | Permissão (mesma chave nos dois lados) | Conta vinda da sessão | Verificado por |
|---|---|---|---|
| Testar conexão (formulário) | não se aplica: app local, sem contas | não se aplica | teste da fase B: nada gravado em arquivo nem no Keychain, senha fora da tela e do erro |
| `preferences.toml` | não se aplica | não se aplica | teste da fase A: só `language`, arquivo 0600, nenhum segredo |

## Riscos

- **Testes por rótulo:** 36 testes de UI buscam texto. Mitigação: idioma fixo no harness e troca tela a tela.
- **Fontes no `egui`:** Inter em tamanho 12 e 13 pode ficar menos nítida que a fonte padrão. Mitigação: conferir
  na fase A antes de seguir; voltar à fonte padrão é uma linha.
- **Peso do binário:** fontes embutidas. Mitigação: medir na A.2, com o teto declarado.
- **Layout no `egui`:** barra de abas com aba ativa "colada" no conteúdo e controle segmentado são desenho à mão.
  Mitigação: o componente nasce na fase A, com teste.
- **Tamanho da FEAT:** toca os 15 arquivos da UI. Mitigação: cinco fases com entrega própria.

## Fora desta branch

- M7: filtros, navegação por chave estrangeira, histórico e tabelas na paleta.
- Seletor de tema dentro do app.
- Terceiro idioma.
