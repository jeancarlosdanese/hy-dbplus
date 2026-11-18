# @file: docs/features/FEAT-016-versao-e-instaladores/00-brief/brief.md

# FEAT-016: versão e instaladores

## Problema

Pedido do Jean em 18/11/2026: "além do ajuste da versão deve também gerar os instaladores para Mac, Linux e
Windows. Crie um script para que isso aconteça."

Hoje:

- A versão está parada em `0.1.0` (`[workspace.package]` do `Cargo.toml`) desde o M0, e o repositório não tem tag.
  O `.claude/process.json` não declara `version` nem `bump`, então a entrega do plugin faz o merge e não sobe a
  versão (as entregas da FEAT-001 à FEAT-015 saíram sem versão).
- Instalador só existe para o macOS: `cargo xtask bundle` monta o `.app` e o `.dmg` (FEAT-014), na máquina do Jean.
  Linux e Windows só compilam e rodam os testes no CI (job `build` do `quality-gate.yml`); ninguém recebe um
  pacote dessas duas plataformas.
- No Windows, o `hy-dbplus.exe` abre junto uma janela de console: o app não declara o subsistema gráfico.

## Objetivo

1. **KR1:** a entrega de uma FEAT sobe a versão sozinha. Depois do merge na `main`, o comando `bump` troca a versão
   em todos os lugares, a trava aceita o commit só da versão e a tag `v<versão>` sai nele. A primeira é a `0.2.0`,
   na entrega desta FEAT.
2. **KR2:** o push da tag `v<versão>` (do Jean) gera, sem passo manual, uma Release no GitHub com os instaladores
   das três plataformas e os checksums: `.dmg` (macOS universal), `.deb` e `.tar.gz` (Linux x86_64) e `.zip`
   (Windows x86_64).
3. **KR3:** os três pacotes se montam pelo mesmo comando, `cargo xtask bundle`, cada um no próprio sistema, e as
   partes puras (nomes, arquivo `control`, `.desktop`, árvore do pacote, versão) têm teste que roda em qualquer
   sistema.

## Escopo

### Must

- [ ] Arquivo `VERSION` na raiz (primeira linha: a versão), lido pela trava do plugin, e `cargo xtask version
      <x.y.z>`, que troca a versão no `VERSION`, no `[workspace.package]` do `Cargo.toml` e nas crates do workspace
      no `Cargo.lock`, sem tocar outra linha.
- [ ] O comando recusa versão inválida (fora de `x.y.z`) e versão que não sobe; um teste confere que `VERSION` e
      `Cargo.toml` têm a mesma versão (o portão pega o descompasso).
- [ ] `.claude/process.json` com `version` (arquivo, caminhos versionados, tag `v{version}`) e `bump` (`cargo xtask
      version {version}`); regra de versão no `CLAUDE.md`: FEAT sobe o minor, ajuste sem FEAT sobe o patch (padrão
      do plugin).
- [ ] `cargo xtask bundle` no Linux: `.deb` (binário em `/usr/bin`, atalho `.desktop` no menu, ícone de 256 px no
      tema `hicolor`) e `.tar.gz` com o binário, o ícone, o `.desktop` e a licença.
- [ ] `cargo xtask bundle` no Windows: `.zip` com o `hy-dbplus.exe` e a licença; o app abre sem janela de console.
- [ ] macOS como hoje (`.dmg`), com o nome no mesmo padrão dos outros.
- [ ] Workflow `release.yml`, disparado pela tag `v*`: confere que a tag é a versão do `VERSION`, monta os três
      pacotes no sistema de cada um, calcula o `SHA256SUMS` e cria a Release com tudo anexado.

### Should

- [ ] A CLI (`hy-dbplus-cli`) junto no `.deb`, no `.tar.gz` e no `.zip`.
- [ ] A Release nasce como rascunho: o Jean revê e publica (os pacotes não têm assinatura de código).

### Could

- [ ] Ícone embutido no `.exe` (precisa de um recurso do Windows compilado; hoje o ícone da janela já vem do PNG
      embutido em tempo de execução).

### Won't

- Instalador `.msi` ou `.exe` com assistente: decisão do Jean em 18/11/2026, sem WiX (ferramenta .NET, licença a
  conferir). Pode voltar numa FEAT própria, com a comparação WiX × NSIS × Inno Setup.
- Assinatura de código no Windows e notarização no CI do macOS: exigem certificado e segredos no GitHub. O `.dmg`
  segue ad hoc (o Gatekeeper pede botão direito → Abrir) e o Windows mostra o aviso do SmartScreen.
- AppImage, Flatpak, `.rpm`, Homebrew, winget, ARM no Linux e no Windows.
- Compilação cruzada a partir do Mac: cada pacote sai no sistema dele, no CI.

## Reconhecimento do código

| O que já existe | Caminho | Decisão |
|---|---|---|
| Versão única do workspace, herdada por todas as crates | `Cargo.toml` (`[workspace.package] version = "0.1.0"`) | manter como fonte do Cargo; o `VERSION` fica igual, conferido por teste |
| `--version` da CLI e `CFBundleShortVersionString` do `.app` | `CARGO_PKG_VERSION`; `xtask/src/bundle.rs` (`plist`) | reaproveitar: seguem a versão do workspace sem mudança |
| `cargo xtask` com tabela de comandos e `cargo metadata` | `xtask/src/main.rs` | estender: comando `version` e o bundle por sistema |
| Bundle do macOS (build, `lipo`, ícone, `Info.plist`, assinatura, `.dmg`) | `xtask/src/bundle.rs` | estender: o `run` escolhe o sistema; o do macOS não muda, só o nome do `.dmg` |
| Ícone em PNG 256 px com alfa | `packaging/icon-256.png` (FEAT-014) | reaproveitar no `.deb`, no `.tar.gz` e no `.desktop` |
| Dependências de sistema do eframe no Linux | `.github/workflows/quality-gate.yml`, job `build` | repetir no `release.yml` (mesma lista) |
| Regra de versão e tag da entrega | plugin process 0.16.1: skill quality (delivery.md, versão e tag na entrega) e a trava (bash_guard.py: `version`, `bump`, `bump_only`) | adotar: a trava lê a versão de um JSON ou da primeira linha de um texto, por isso o `VERSION` |
| Licença MIT | `LICENSE` | vai nos pacotes |

Duplicidade ou conflito encontrado: a trava não lê a versão do `Cargo.toml` (só JSON com chave `version` ou a
primeira linha de um texto). Resolve-se com o `VERSION` espelhando o `Cargo.toml`, os dois escritos pelo mesmo
comando e conferidos por teste.

## Restrições

- **Rust only no build:** a lógica fica no `xtask` (Rust); o workflow só chama `cargo xtask` e as ferramentas do
  sistema de cada runner (`dpkg-deb` e `tar` no Ubuntu, `tar` no Windows, as do Xcode no macOS). Nenhuma
  dependência nova no Cargo.
- **Never panic** também no `xtask`, como hoje.
- **Push e tag publicados são do Jean:** a IA cria a tag local na entrega; o push dela, que dispara a Release, é dele.
- `xtask/`, `packaging/`, `Cargo.toml`, `Cargo.lock`, `apps/*/src/` e `.github/` são sensíveis ou protegidos: passam
  pelo revisor.
- A chave `version` do `process.json` liga a trava de versão assim que é gravada: entra no último commit da branch,
  com o `VERSION` em `0.1.0`, para que a primeira subida seja a da entrega desta FEAT.

## Decisões do mantenedor (18/11/2026)

- Script de versão e instaladores para macOS, Linux e Windows ("Crie um script para que isso aconteça").
- Windows sem WiX: `.zip` com o `.exe` ("Ok, vamos sem WiX").
- Ordem: entregar a FEAT-015 sem versão e fazer a versão e os instaladores nesta FEAT; primeira versão `0.2.0`
  ("sim").

## Decisões abertas

Nenhuma.

## Referências

- FEAT-014 (empacotamento macOS): `docs/features/FEAT-014-empacotamento-macos/`.
- Plugin process 0.16.1: `skills/quality/delivery.md` (versão e tag na entrega) e `skills/feat-new/project-contract.md`
  (quanto sobe).
- `docs/BRIEF.md`, M8.
