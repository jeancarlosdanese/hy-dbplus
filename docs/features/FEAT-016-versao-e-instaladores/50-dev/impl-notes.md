# @file: docs/features/FEAT-016-versao-e-instaladores/50-dev/impl-notes.md

# Notas de implementação: FEAT-016

> Plano aprovado pelo Jean em 18/11/2026 ("pode seguir"; sem tela, uma aprovação).

## Decisões de implementação

- **`VERSION` na raiz:** a trava do plugin lê a versão da primeira linha de um texto (ou da chave `version` de um
  JSON); o `Cargo.toml` não serve. O `Cargo.toml` continua a fonte para o Cargo (`CARGO_PKG_VERSION`, `Info.plist`),
  e os dois são escritos pelo mesmo comando e conferidos por teste.
- **`Cargo.lock` editado pelo `xtask`, não pelo `cargo update`:** o comando troca `version = "<antiga>"` só nos
  blocos `[[package]]` cujo `name` é membro do workspace (lista do `cargo metadata --no-deps`). Assim a trava vê só
  linhas de versão (`bump_only`) e nenhuma dependência de terceiro é resolvida de novo. Depois, `cargo metadata
  --locked` confere que o lock bate com os manifestos.
- **Nome dos pacotes:** `hy-dbplus-<versão>-<sistema>-<arquitetura>.<ext>` (`macos-universal.dmg`,
  `macos-arm64.dmg`, `linux-x86_64.deb`, `linux-x86_64.tar.gz`, `windows-x86_64.zip`). O `.dmg` muda de
  `hy-dbplus-<versão>-<arch>.dmg` para esse padrão.
- **`.deb` sem `dpkg-shlibdeps`:** o `control` declara as dependências fixas (`DEB_DEPENDS` em
  `xtask/src/package.rs`: libc, runtime do Rust, GL e EGL, Wayland, X11 e xkbcommon). As de GL, Wayland e X11 são
  abertas por `dlopen` e não aparecem no `ldd`; o CI instala o `.deb` num Debian limpo para conferir que o `apt`
  resolve todas. O pacote nasce com `dpkg-deb --build --root-owner-group`.
- **`.zip` pelo `tar` do sistema:** no Windows 10 em diante o `tar` é o bsdtar e grava zip (`tar -a -c -f`); no
  Linux, o `.tar.gz` sai pelo `tar -czf`. Nenhuma crate de compressão.
- **Release em rascunho:** os pacotes não têm assinatura de código; o Jean revê a Release e publica.
- **Conferência da tag:** `cargo xtask version --check <tag>` falha se a tag não é `v` + o `VERSION`; o
  `release.yml` roda isso antes de montar qualquer pacote.

## Princípios

- A versão publicada nunca desce nem se repete (`cargo xtask version` recusa; a trava também). 18/11/2026, Jean
  ("sim" ao plano de versão).
- Cada pacote sai no próprio sistema; nada de compilação cruzada no Mac.

## Plano

Passos marcáveis, por fase. Cada fase fecha com teste, notas e o "pode commitar" do Jean.

### Fase A: versão

- [x] A.1 `VERSION` na raiz com `0.1.0`.
- [x] A.2 `xtask/src/version.rs`: as partes puras (validar `x.y.z`, comparar, trocar a versão no texto do
      `Cargo.toml` só em `[workspace.package]`, trocar no `Cargo.lock` só nos membros do workspace) e o `run`, que
      lê, valida, grava os três arquivos e confere com `cargo metadata --locked`. Comando `version` na tabela do
      `main.rs`, com `--check <tag>`.
- [x] A.3 Testes: troca correta e só nas linhas da versão (dependência de terceiro com a mesma versão fica);
      recusas (`0.1.0`, `0.0.9`, `0.3`, `v0.3.0`, `0.3.0-beta`, vazio) sem gravar nada; `--check` com tag certa,
      errada e sem `v`; e o teste de compasso, que lê o `VERSION` e o `Cargo.toml` reais da raiz.
- [x] A.4 Portão `--full`, revisor da fase e notas (três passadas; a última sem achados).

### Fase B: pacotes do Linux e do Windows

- [x] B.1 `bundle.rs`: o `run` escolhe pelo sistema (`macos`, `linux`, `windows`; outro sistema recusa com erro
      claro). O macOS não muda, salvo o nome do `.dmg`.
- [x] B.2 Linux: build release do app e da CLI, árvore do `.deb` (`DEBIAN/control`, `/usr/bin`,
      `/usr/share/applications/br.com.hy.dbplus.desktop`, `/usr/share/icons/hicolor/256x256/apps/hy-dbplus.png`,
      `/usr/share/doc/hy-dbplus/copyright`), `dpkg-deb --build --root-owner-group`, e o `.tar.gz` com os binários, o
      ícone, o `.desktop` e a licença.
- [x] B.3 Windows: build release do app e da CLI, `.zip` com os dois `.exe` e a licença; `apps/hy-dbplus/src/main.rs`
      ganha `windows_subsystem = "windows"` no release (sem janela de console); como o stderr some nesse modo, o erro
      que impede a abertura vai também para `error.log`, ao lado do `connections.toml`.
- [x] B.4 Testes das partes puras, em qualquer sistema: nomes dos pacotes, texto do `control` (pacote, versão,
      arquitetura, dependências, descrição), `.desktop` (nome, `Exec`, `Icon`, categorias), árvore do `.deb` e do
      `.tar.gz` montada numa pasta temporária, argumentos do bundle por sistema; teste do atributo de subsistema no
      `main.rs`.
- [x] B.5 Portão `--full`, revisor da fase e notas (duas passadas; o baixo da segunda, a lista antiga nas notas, corrigido sem nova passada).

### Fase C: release no CI e a versão ligada na entrega

- [x] ⏸️ C.0 (cancelado em 18/11/2026: a entrega é toda local, e o CI roda no push da `main` e da tag, que é do
      Jean; ver "Desvios") Mantenedor: push da branch e pull request, para o `release.yml` rodar nos três sistemas
      sem criar Release:
      ```bash
      git push -u origin feat/FEAT-016-versao-e-instaladores
      gh pr create --fill --draft
      ```
      Saída esperada: os jobs `macos`, `linux` e `windows` verdes, com os pacotes nos artefatos do run e o
      `dpkg-deb --info` no log do Linux. O job `release` fica pulado (não é tag).
- [x] C.1 `.github/workflows/release.yml`: gatilhos `push` de tag `v*`, e `push` da `main` e `pull_request` que
      tocam `xtask/`, `packaging/`, `apps/hy-dbplus/`, `apps/hy-dbplus-cli/`, `VERSION`, `Cargo.toml`, `Cargo.lock` ou o próprio workflow; job `check` (`cargo xtask version --check` com o nome
      da tag, só na tag); jobs `macos` (`--universal`), `linux` (dependências do eframe, `.deb` e `.tar.gz`,
      `dpkg-deb --info` e `--contents`, e o `.deb` instalado por `apt install` num contêiner `debian:bookworm-slim`
      limpo, que prova que todas as dependências se resolvem, e o app aberto por alguns segundos sob `xvfb-run` no mesmo
      contêiner, que pega biblioteca nova que o `winit` ou o `glutin` passem a abrir; o `ldd` não vê as abertas por
      `dlopen`) e `windows`, cada um subindo os pacotes como artefato; job `release`, só na
      tag, com `contents: write`, que baixa os artefatos, gera o `SHA256SUMS` e roda `gh release create --draft
      --generate-notes`. `permissions: contents: read` no topo.
- [x] C.2 `.claude/process.json`: `version` (`file: VERSION`, `paths` com `^(apps|crates|xtask|packaging)/` e
      `^Cargo\.(toml|lock)$`, `tag: v{version}`) e `bump: cargo xtask version {version}`; o `comment` atualizado.
      Antes, conferir nos testes do plugin (`hooks/bash_guard_test.py`) o caminho "VERSION novo na `main` sem tag →
      commit da versão pela entrega → tag", para a primeira entrega não travar.
- [x] C.3 `CLAUDE.md`: a regra de versão ("FEAT sobe o minor, ajuste sem FEAT sobe o patch, a partir da
      FEAT-016"), a frase da entrega sem o "até o M8", e os comandos `cargo xtask version` e o bundle das três
      plataformas. `docs/BRIEF.md`: o M8 com os instaladores.
- [x] C.4 Portão `--full`, revisor da fase e notas (duas passadas); depois, fechamento (`feat-close`) com a revisão da branch
      inteira. Na entrega, a `0.2.0` e a tag `v0.2.0`; o push da tag, que cria a Release, é do Jean.

## Desvios do plano

- **A.2, conferência com o Cargo:** o `cargo metadata --locked --offline` depois de gravar ficou no `main.rs` do
  xtask (junto do outro `cargo metadata`), não no `version::run`, que segue sem rodar processo e testável numa pasta
  temporária: ele recebe a conferência como função. A conferência usa `--offline` (o bump não vai à rede); se
  falhar, por lock fora de compasso ou por pacote fora do cache do cargo, os três arquivos voltam como estavam e a
  causa vem no erro. Na entrega o portão completo roda antes, com o `cargo-deny` enchendo o cache.
- **A.2, achados do revisor (lote único):** o `VERSION` só aceita `x.y.z` com um fim de linha opcional (antes
  aceitava espaços e linhas a mais, e a regravação quebraria o `bump_only` da trava); a troca no `Cargo.toml` e no
  lock mexe só no valor dentro da linha, preservando recuo e espaço no fim; falha depois da primeira gravação
  devolve os originais. Testes provados contra o código sem cada conserto.
- **A.2, 2ª passada do revisor:** com o `VERSION` estrito, o checkout do Windows no CI (`core.autocrlf=true`)
  traria `0.1.0\r\n` e o teste de compasso cairia (médio). Entrou o `.gitattributes` com `VERSION text eol=lf`,
  que mantém a recusa de `\r\n`. E a gravação passou a ir para um temporário na mesma pasta com `rename` por
  cima: o `fs::write` direto truncava o arquivo antes de escrever, e uma falha no meio deixaria o lock vazio com a
  mensagem "nada mudou" (baixo).

- **B, módulo novo:** Linux e Windows ficaram em `xtask/src/package.rs`; o `bundle.rs` segue dono do macOS e do
  `run`, que escolhe pelo sistema. Fora do macOS, `--universal` e `--no-dmg` são recusados.
- **B.1, nome do `.dmg` sem `--universal`:** usava `arm64` fixo, mesmo num Mac Intel; agora é a arquitetura da
  máquina. Os documentos da FEAT-014 citam o nome antigo e ficam como histórico.
- **B.2, atalho com o nome do `app_id`:** `br.com.hy.dbplus.desktop`, com `StartupWMClass` igual: no Wayland o
  compositor casa a janela com o atalho pelo `app_id`, e o ícone da barra só aparece assim.
- **B.2, dependências do `.deb`:** sem GTK (o eframe aqui não tem diálogo de arquivo); a lista tem dono único, o
  `DEB_DEPENDS` de `xtask/src/package.rs` (12 pacotes, com o motivo de cada grupo no comentário).
- **B.2, `Maintainer`:** `Hyberica <página do projeto>`, sem e-mail: nenhum endereço pessoal vai no pacote.
- **B.3, `tar.exe` do `System32`:** o `tar` do Git no PATH do runner é o GNU, que não grava zip; o bsdtar do sistema
  grava.
- **B.3, `windows_subsystem` só no release:** o build de debug segue com console para os logs.

- **C.2, a 0.2.0 sobe na branch, não depois do merge:** lendo a `version_gate` da trava, o commit da versão na
  `main` seria recusado: ela exige que a versão do último commit (0.1.0, chegada com o merge) já tenha a tag
  `v0.1.0`, que nunca existiu (os testes do plugin partem de uma versão já marcada). A trava prevê a outra forma:
  "a tag que falta vai na ponta da principal (o commit da versão, ou o de merge quando a branch já subiu a
  versão)". Por isso o `cargo xtask version 0.2.0` rodou na branch, e a entrega desta FEAT faz o merge e cria a
  `v0.2.0` no commit de merge, sem tag `v0.1.0` artificial. Da próxima entrega em diante vale o fluxo normal
  (merge, commit da versão, tag). O merge passa: antes dele a `main` não tem `VERSION`, e a trava não cobra nada.
- **C.1, Linux no `ubuntu-22.04`:** montado no 24.04, o binário exigiria a glibc 2.39 e não abriria no Debian 12
  (2.36) nem no Ubuntu 22.04; com a 2.35 roda de 2022 em diante.
- **C.1, conferência do `.deb`:** num contêiner `debian:bookworm-slim`, `apt install` do `.deb` (prova que o
  `Depends` se resolve), `hy-dbplus-cli --version` e o app aberto 15 s sob `xvfb-run`, com o Mesa fazendo o papel
  do driver do desktop e a senha só em memória. Saída antes dos 15 s reprova.
- **C.1, ações de artefato:** `upload-artifact@v7` e `download-artifact@v8` (as últimas, consultadas na API do
  GitHub em 18/11/2026; Node 24, com `pattern` e `merge-multiple`); no fechamento, todas as ações do `release.yml`
  foram fixadas por SHA e o `rust-cache` saiu (abaixo).
- **C.1, notas da Release:** as geradas pelo GitHub, precedidas do aviso de pacotes sem assinatura, que manda
  conferir pelo `SHA256SUMS` (anexo).
- **C, achados do revisor (lote único, todos baixos):** o `CLAUDE.md` mandava subir a versão em toda entrega, mas a
  trava recusa versão nova sem mudança nos `paths` desde a tag: a regra agora diz isso. O job `release`, reexecutado
  depois de falha no meio, quebrava no `create` (o rascunho já existia): agora sobe os anexos por cima. As ações de
  artefato estavam na v4 (Node 20). E o aceite e as notas diziam que esta entrega provava o commit da versão na
  `main`, que ela não percorre: o revisor simulou com as funções da trava (merge desta FEAT, tag no commit de merge,
  merge seguinte, `cargo xtask version 0.3.0`, commit e tag passam; commit na `main` antes da tag é recusado), e a
  prova real fica para a próxima entrega.
- **Fechamento, revisão da branch inteira (1 médio, 5 baixos, lote único):** o `.tar.gz` gravava o dono do
  usuário que montou (no runner, uid 1001): extraído com `sudo`, o binário em `/opt` ficaria de um usuário comum —
  médio; o `tar` agora grava `--owner=0 --group=0 --numeric-owner`, e o CI reprova entrada que não seja 0/0. Como a
  entrega é merge local, o `release.yml` só montava pacote em pull request ou na tag: agora roda também no push da
  `main` que toca o empacotamento (o `check` e o `release` passaram a olhar `refs/tags/v`), e o próprio
  `release.yml` entrou nos `paths` do `version` (conserto só nele pode subir versão e tag nova). As ações do
  `release.yml` ficaram fixadas por SHA (os binários vão para a Release), sem o `rust-cache`, e a imagem do Debian
  por digest. A página do projeto no `.deb` vem do `repository` do `cargo metadata` (sem cópia no xtask). O brief,
  o AC-006, o C.1 e o comentário do alias em `.cargo/config.toml` foram alinhados. A passada sobre o delta achou um baixo (uma frase das notas
  ainda dizia que as ações seguiam as do `quality-gate.yml`), corrigido sem nova passada.
- **C, 2ª passada (um baixo, corrigido sem nova passada):** o reenvio por cima também regravaria os anexos de uma
  Release já publicada (reexecução do run ou tag movida); agora só regrava rascunho, e publicada falha dizendo
  que versão publicada não se regrava.

- **C.0 cancelado:** o Jean corrigiu o fluxo em 18/11/2026: "entrega" é toda local (portão, merge, versão, commit da
  versão, tag e o `.dmg`, na ordem do `CLAUDE.md`) e o push é dele; não há pull request nem CI antes. Os critérios que só o CI prova (AC-006,
  AC-007 e AC-010) ficam abertos com o motivo, e a conferência deles é o run do `release.yml` no push. A entrega
  também monta o `.dmg` local (`cargo xtask bundle`), regra registrada no `CLAUDE.md`.

## Testes

Fase A (18/11/2026), por teste e no repositório:
- Parser de versão: `x.y.z` aceito; vazio, `0.3`, `0.3.0.1`, `v0.3.0`, `0.3.0-beta`, `0.03.0`, letras e espaço
  recusados; ordem numérica (`0.10.0` > `0.9.9`).
- Subida em textos de teste: só a linha de `[workspace.package]` muda no `Cargo.toml` (a dependência com a mesma
  versão fica) e só os membros no lock (o terceiro com a mesma versão fica); fins de linha `\r\n` preservados.
- Recusas sem gravar nada: versão igual, menor ou inválida; `VERSION` e `Cargo.toml` fora de compasso; membro que
  falta no lock ou com outra versão; `Cargo.toml` sem `[workspace.package]`; argumentos errados.
- `--check`: tag certa passa; outra versão, sem `v` e com sufixo falham.
- Compasso real: o teste lê o `VERSION`, o `Cargo.toml` e o `Cargo.lock` da raiz; com o `VERSION` em 0.2.0 ele
  falhou dizendo os dois valores (prova feita e desfeita).
- No repositório: `cargo xtask version 0.2.0` mudou 9 linhas do lock (os 9 membros) e 1 do `Cargo.toml`, o
  `cargo metadata --locked` e o build `--locked` do xtask passaram, `--check v0.2.0` passou e `--check v0.3.0`
  falhou; tudo desfeito. `0.0.9`, `abc` e sem argumento recusados.
- Depois do lote do revisor: `VERSION` com linha a mais, espaço ou `\r\n` recusado; sem fim de linha, o novo
  sai sem; recuo com tab e espaço no fim preservados; conferência que falha devolve os três arquivos (os dois
  testes falham sem o conserto). Subida real repetida e desfeita.
- Portão completo verde.

Fase B (18/11/2026), por teste e no Mac:
- Nomes dos pacotes e arquiteturas (`x86_64`, `arm64`; outra recusada), `control` do `.deb` (campos, formato do
  dpkg: fim em `\n`, descrição longa com espaço, sem linha vazia nem espaço no fim), `.desktop` (campos e nome).
- Árvore do `.deb` numa pasta temporária: os seis arquivos, a árvore recriada (sobra de antes some), binários 0755,
  o resto 0644, pastas 0755, `Installed-Size` em KiB. Pastas do `.tar.gz` e do `.zip` com os arquivos certos.
  Entrada que falta é recusada.
- `windows_subsystem` no `main.rs` do app: o teste falha sem o atributo (prova feita e desfeita).
- macOS no Mac: `cargo xtask bundle` montou `hy-dbplus-0.1.0-macos-arm64.dmg`, com 0.1.0 no `Info.plist` e a
  assinatura ad hoc válida (`codesign -v`); argumento desconhecido recusado.
- Portão completo verde.

Fase C (18/11/2026), local:
- `cargo xtask version 0.2.0` no repositório: 9 linhas do lock, 1 do `Cargo.toml` e o `VERSION`; portão completo
  verde com a 0.2.0.
- `release.yml` lido como YAML (Ruby): cinco jobs, escrita só no `release`.
- `.claude/process.json` válido, com `version` e `bump`.

Depois da entrega (18/11/2026): o push do Jean rodou o `release.yml` na `main` e na tag `v0.2.0`, os dois verdes:
`.dmg` universal, `.deb` (Version 0.2.0, instalado num Debian 12 limpo, `hy-dbplus-cli 0.2.0`, app aberto 15 s sob
Xvfb), `.tar.gz` com dono 0/0 e `.zip`; a Release `hy-dbplus 0.2.0` nasceu em rascunho com os cinco arquivos. O
`.dmg` abriu no Mac do Jean. Na entrega do fix seguinte, a trava aceitou o commit só da versão (`d5a5f52`, 0.2.1).

**Não conferido (antes da entrega; os itens de CI acima já foram conferidos):** o `release.yml` rodando (roda no push da `main` e da tag, do Jean, depois da entrega); o `.deb`, o `.tar.gz` e o `.zip` montados de verdade (esta máquina é macOS; montam no CI, fase
C); o `--universal` (falta o target x86_64 neste Mac; monta no CI); o checkout do Windows com o `.gitattributes` (o `VERSION` ainda não foi para o remoto; confere
no CI do push da `main`); a falha de gravação depois de truncar (disco cheio), que não se simula em
teste: o teste cobre a falha no segundo arquivo, com o primeiro devolvido e sem temporário sobrando; a subida dentro da entrega, com a trava do plugin (esta entrega só faz o merge e a tag no commit de merge; o commit da
versão na `main` foi conferido por simulação com as funções da trava e roda de verdade na próxima entrega).

## Segurança (fechamento)

Sem rota nem tela nova. O que importa aqui é o `release.yml` (token e permissões) e o conteúdo dos pacotes, no
AC-011 e no revisor: escrita (`contents: write`) só no job `release`, token do próprio run, tag por variável de
ambiente, ações por SHA e imagem por digest, pacotes sem arquivo de conexão, preferência ou senha, `.tar.gz` com
dono root. Verificado pelo revisor em cada fase e na branch inteira.

## Riscos

- **Dependências do `.deb` fixas:** uma biblioteca carregada em tempo de execução (Wayland, X11, GL) pode faltar num
  Ubuntu mínimo. Mitigação: a lista no `control` vem do código do `winit` e do `glutin`, e o CI instala o `.deb`
  num Debian limpo; o `.tar.gz` não declara nada.
- **Primeira entrega com versão:** a trava passa a cobrar versão e tag com o `process.json` novo; um caminho que não
  esteja nos testes do plugin pode recusar o commit da versão. Mitigação: o C.2 confere nos testes antes, e a
  recusa não se contorna (vira pergunta ao Jean).
- **Runner `macos-latest` com o target x86_64:** o `--universal` exige `rustup target add x86_64-apple-darwin` no
  job. Mitigação: passo explícito no workflow.
- **Sem assinatura:** Gatekeeper e SmartScreen avisam na primeira abertura; o rascunho da Release diz isso.

## Fora desta branch

- Instalador `.msi` ou com assistente no Windows (sem WiX, decisão de 18/11/2026).
- Assinatura e notarização no CI (certificados e segredos no GitHub).
- README do produto (M8) e o texto da Release além das notas geradas.
- AppImage, Flatpak, `.rpm`, Homebrew, winget, ARM no Linux e no Windows.
