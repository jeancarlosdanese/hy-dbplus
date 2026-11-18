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
- **`.deb` sem `dpkg-shlibdeps`:** o `control` declara as dependências fixas (`libc6`, `libgcc-s1`, `libgl1`,
  `libxkbcommon0`, `libgtk-3-0`), conferidas no CI com `ldd` e `dpkg-deb --info`; o pacote nasce com
  `dpkg-deb --build --root-owner-group`.
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

- [ ] B.1 `bundle.rs`: o `run` escolhe pelo sistema (`macos`, `linux`, `windows`; outro sistema recusa com erro
      claro). O macOS não muda, salvo o nome do `.dmg`.
- [ ] B.2 Linux: build release do app e da CLI, árvore do `.deb` (`DEBIAN/control`, `/usr/bin`,
      `/usr/share/applications/hy-dbplus.desktop`, `/usr/share/icons/hicolor/256x256/apps/hy-dbplus.png`,
      `/usr/share/doc/hy-dbplus/copyright`), `dpkg-deb --build --root-owner-group`, e o `.tar.gz` com os binários, o
      ícone, o `.desktop` e a licença.
- [ ] B.3 Windows: build release do app e da CLI, `.zip` com os dois `.exe` e a licença; `apps/hy-dbplus/src/main.rs`
      ganha `#![cfg_attr(windows, windows_subsystem = "windows")]` (sem janela de console; o erro de abrir a janela
      continua no stderr, que no Windows gráfico não aparece: anotado em "Não conferido").
- [ ] B.4 Testes das partes puras, em qualquer sistema: nomes dos pacotes, texto do `control` (pacote, versão,
      arquitetura, dependências, descrição), `.desktop` (nome, `Exec`, `Icon`, categorias), árvore do `.deb` e do
      `.tar.gz` montada numa pasta temporária, argumentos do bundle por sistema; teste do atributo de subsistema no
      `main.rs`.
- [ ] B.5 Portão `--full`, revisor da fase e notas.

### Fase C: release no CI e a versão ligada na entrega

- [ ] ⏸️ C.0 Mantenedor: depois do commit da fase, push da branch e abertura do pull request, para o `release.yml`
      rodar nos três sistemas sem criar Release:
      ```bash
      git push -u origin feat/FEAT-016-versao-e-instaladores
      gh pr create --fill --draft
      ```
      Saída esperada: os jobs `macos`, `linux` e `windows` verdes, com os pacotes nos artefatos do run e o
      `dpkg-deb --info` no log do Linux. O job `release` fica pulado (não é tag).
- [ ] C.1 `.github/workflows/release.yml`: gatilhos `push` de tag `v*` e `pull_request` que toca `xtask/`,
      `packaging/`, `apps/hy-dbplus/` ou o próprio workflow; job `check` (`cargo xtask version --check` com o nome
      da tag, só na tag); jobs `macos` (`--universal`), `linux` (dependências do eframe, `.deb` e `.tar.gz`,
      `dpkg-deb --info` e `--contents`) e `windows`, cada um subindo os pacotes como artefato; job `release`, só na
      tag, com `contents: write`, que baixa os artefatos, gera o `SHA256SUMS` e roda `gh release create --draft
      --generate-notes`. `permissions: contents: read` no topo.
- [ ] C.2 `.claude/process.json`: `version` (`file: VERSION`, `paths` com `^(apps|crates|xtask|packaging)/` e
      `^Cargo\.(toml|lock)$`, `tag: v{version}`) e `bump: cargo xtask version {version}`; o `comment` atualizado.
      Antes, conferir nos testes do plugin (`hooks/bash_guard_test.py`) o caminho "VERSION novo na `main` sem tag →
      commit da versão pela entrega → tag", para a primeira entrega não travar.
- [ ] C.3 `CLAUDE.md`: a regra de versão ("FEAT sobe o minor, ajuste sem FEAT sobe o patch, a partir da
      FEAT-016"), a frase da entrega sem o "até o M8", e os comandos `cargo xtask version` e o bundle das três
      plataformas. `docs/BRIEF.md`: o M8 com os instaladores.
- [ ] C.4 Portão `--full`, revisor da fase e notas; depois, fechamento (`feat-close`) com a revisão da branch
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

**Não conferido:** o checkout do Windows com o `.gitattributes` (o `VERSION` ainda não foi para o remoto; confere
no CI do primeiro push da branch); a falha de gravação depois de truncar (disco cheio), que não se simula em
teste: o teste cobre a falha no segundo arquivo, com o primeiro devolvido e sem temporário sobrando; a subida dentro da entrega, com a trava do plugin (é a da própria entrega desta FEAT, na fase
C).

## Segurança (fechamento)

Sem rota nem tela nova. O que importa aqui é o `release.yml` (token e permissões) e o conteúdo dos pacotes, no
AC-011 e no revisor.

## Riscos

- **Dependências do `.deb` fixas:** uma biblioteca carregada em tempo de execução (Wayland, X11, GL) pode faltar num
  Ubuntu mínimo. Mitigação: `ldd` no log do CI e a lista no `control`; o `.tar.gz` não declara nada.
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
