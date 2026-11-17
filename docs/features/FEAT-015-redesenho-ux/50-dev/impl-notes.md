# @file: docs/features/FEAT-015-redesenho-ux/50-dev/impl-notes.md

# Notas de implementação: FEAT-015

> Protótipo aprovado pelo Jean em 17/11/2026 ("pode seguir"), como desenhado: ambiente em etiqueta e faixa fina.
> Aceite e plano aprovados no mesmo dia ("pode seguir"). Fase A implementada, aguardando revisão e o "pode commitar".

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

Objetivo: tela de conexões, formulário, remover, com teclado e Testar conexão (telas 1 a 4).

- [ ] B.0 Detalhar os passos da fase.

### Fase C: workspace

Objetivo: barra superior, faixa de bancos, árvore, abas, paleta e confirmações (telas 5, 6 e 16).

- [ ] C.0 Detalhar os passos da fase.

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
- Portão completo verde e `cargo deny check advisories` ok.

**Não conferido:**
- Nitidez da Inter em 12 e 13 px e o visual geral no app: a captura de tela não tem permissão nesta sessão. Fica
  para a conferência do Jean.
- Idioma do sistema real em outra língua (só por teste da função de locale).
- Linux e Windows (só o rótulo de atalho por teste).

## Segurança (fechamento)

| Rota ou tela | Permissão (mesma chave nos dois lados) | Conta vinda da sessão | Verificado por |
|---|---|---|---|
| Testar conexão (formulário) | não se aplica: app local, sem contas | não se aplica | a preencher na fase B: senha fora de log, erro e arquivo |
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
