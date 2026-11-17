# @file: docs/features/FEAT-015-redesenho-ux/50-dev/impl-notes.md

# Notas de implementação: FEAT-015

> Protótipo aprovado pelo Jean em 17/11/2026 ("pode seguir"), como desenhado: ambiente em etiqueta e faixa fina.
> Aceite e plano aprovados no mesmo dia ("pode seguir"). Fases A (e461e98) e B (1406e00) commitadas. Fase C
> implementada de ponta a ponta, a pedido do Jean, sem commit.

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

- [ ] D.0 Detalhar os passos da fase.

### Fase E: editor SQL

Objetivo: barra do editor, autocomplete, resultados, erro e ROLLBACK (telas 13 a 15).

- [ ] E.0 Detalhar os passos da fase.

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
- Portão completo verde.
- Portão completo verde e `cargo deny check advisories` ok.

**Não conferido:**
- Nitidez da Inter em 12 e 13 px e o visual geral no app: a captura de tela não tem permissão nesta sessão. Fica
  para a conferência do Jean.
- Idioma do sistema real em outra língua (só por teste da função de locale).
- Linux e Windows (só o rótulo de atalho por teste).
- Fase B no app: visual das telas 1 a 4 nos dois temas e idiomas, foco do teclado com o campo de busca ativo, e o
  "Testar conexão" contra um servidor real. Conferência do Jean.
- Fase C no app: visual das telas 5, 6 e 16 nos dois temas e idiomas; a faixa de ambiente com cada tag.

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
