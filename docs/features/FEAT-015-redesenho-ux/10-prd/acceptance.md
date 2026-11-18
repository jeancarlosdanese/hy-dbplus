# @file: docs/features/FEAT-015-redesenho-ux/10-prd/acceptance.md

# Critérios de aceite: FEAT-015

O contrato visual é o protótipo aprovado (`../20-prototipos/prototipo.html`). Os cenários abaixo fixam o que um
teste prova; o que só o olho confere está marcado como conferência visual.

```gherkin
Cenário: idioma segue o sistema
  Dado um sistema em português do Brasil e nenhuma preferência guardada
  Quando o app abre
  Então a interface aparece em português

Cenário: sistema em outro idioma cai no inglês
  Dado um sistema em espanhol, ou um idioma que o app não consegue ler
  Quando o app abre
  Então a interface aparece em inglês
  E o app não entra em pânico

Cenário: trocar o idioma pelo seletor
  Dado a interface em português
  Quando o usuário clica em EN no seletor da barra superior
  Então todos os textos da tela passam para inglês no mesmo instante
  E a escolha vale na próxima abertura do app

Cenário: arquivo de preferências inválido
  Dado um arquivo de preferências corrompido ou com idioma desconhecido
  Quando o app abre
  Então ele usa o idioma do sistema
  E não mostra erro nem entra em pânico

Cenário: nenhum texto misturado
  Dado a interface em um idioma
  Quando se percorrem todas as telas e diálogos
  Então nenhum rótulo, dica ou mensagem do app aparece no outro idioma
  E nome de tabela, tipo do PostgreSQL, SQL e mensagem de erro do servidor ficam como o banco devolve

Cenário: atalho conforme a plataforma
  Dado o app rodando fora do macOS
  Quando um rótulo mostra um atalho
  Então ele mostra Ctrl, não ⌘

Cenário: alteração pendente visível na linha selecionada
  Dado uma célula alterada e ainda não gravada
  Quando a linha dela é selecionada
  Então a célula continua com a cor de pendência, diferente da cor de seleção

Cenário: gravar alterações
  Dado 2 alterações e 1 exclusão pendentes em pedidos
  Quando o usuário pede para revisar e gravar
  Então o diálogo tem o título "Gravar 2 alterações e 1 exclusão em pedidos"
  E com 1 alteração só o título diz "1 alteração", no singular

Cenário: diálogos são modais
  Dado qualquer diálogo aberto (formulário, remover, gravar, DDL, paleta, fechar)
  Quando o usuário clica fora ou aperta Esc
  Então o diálogo fecha sem executar a ação
  E a tela de trás não recebe o clique
  Mas durante a gravação o diálogo não fecha

Cenário: formulário de conexão pelo teclado
  Dado o formulário aberto com dados válidos
  Quando o usuário aperta Enter
  Então a conexão é salva
  E com a porta "54321x" o formulário não salva e mostra o erro junto do campo Porta

Cenário: testar conexão
  Dado o formulário preenchido
  Quando o usuário clica em Testar conexão
  Então o resultado aparece ao lado do botão: o tempo, ou o erro do servidor
  E nada é salvo no arquivo de conexões nem no Keychain
  E a senha não aparece em log, em erro nem em arquivo

Cenário: lista de conexões pelo teclado
  Dado a tela de conexões com uma linha em foco
  Quando o usuário aperta Enter
  Então o app conecta naquela conexão
  E as setas mudam a linha em foco

Cenário: transação abortada
  Dado uma transação aberta no editor e um statement que falha
  Quando o erro aparece
  Então a faixa de aviso oferece executar ROLLBACK
  E a transação aberta sem erro aparece como aviso, não como erro

Cenário: fechar com pendências
  Dado uma aba com alterações não gravadas
  Quando o usuário fecha a aba
  Então a confirmação diz quantas alterações se perdem
  E oferece revisar e gravar, cancelar ou descartar e fechar

Cenário: botões sem função não aparecem
  Dado qualquer tela
  Então não há botão desabilitado com "em breve" ou "não disponível nesta versão"

Cenário: o que já funcionava continua funcionando
  Dado a suíte de testes de antes da FEAT, com os rótulos novos
  Quando ela roda
  Então todos os testes passam
```

## Checklist

Marcados em 18/11/2026: cobertos por teste (e, no AC-004 e no AC-015, pela varredura da fase F, registrada
nas notas). O AC-016 é a conferência visual do Jean.

- [x] AC-001: idioma do sistema em pt abre em português; outro idioma ou ilegível abre em inglês, sem pânico.
      Verificar: teste.
- [x] AC-002: o seletor PT | EN troca os textos na hora e a escolha persiste. Verificar: teste.
- [x] AC-003: preferência inválida cai no idioma do sistema, sem erro. Verificar: teste.
- [x] AC-004: nenhum texto do app misturado: os dois catálogos têm as mesmas chaves, e nenhum literal de interface
      fica fora do catálogo. Verificar: teste (paridade do catálogo) e varredura por `grep` no fechamento.
- [x] AC-005: rótulo de atalho mostra Ctrl fora do macOS. Verificar: teste.
- [x] AC-006: pendência e seleção têm cores diferentes e a pendência prevalece na célula. Verificar: teste dos
      tokens e conferência visual.
- [x] AC-007: título do diálogo de gravação com contagem e plural corretos nos dois idiomas. Verificar: teste.
- [x] AC-008: todos os diálogos são modais; Esc e clique fora cancelam, salvo durante a gravação. Verificar: teste.
- [x] AC-009: Enter salva o formulário válido; porta inválida não salva e mostra o erro no campo. Verificar: teste.
- [x] AC-010: Testar conexão mostra tempo ou erro, não salva nada e não expõe a senha. Verificar: teste com o
      driver falso.
- [x] AC-011: setas e Enter na lista de conexões. Verificar: teste.
- [x] AC-012: transação abortada oferece ROLLBACK; transação aberta é aviso. Verificar: teste.
- [x] AC-013: confirmação de fechar com a contagem e as três saídas. Verificar: teste.
- [x] AC-014: nenhum botão morto na tela. Verificar: teste.
- [x] AC-015: nenhuma cor nem espaçamento fora dos tokens em `crates/ui-egui/src/`, salvo o `theme.rs`.
      Verificar: varredura por `grep` no fechamento.
- [ ] AC-016 (motivo: conferência do Jean): as 17 telas batem com o protótipo nos dois temas e nos dois idiomas.
      Verificar: conferência visual do Jean no app. A captura de tela do app não tem permissão nas sessões da IA;
      fica em "Cabe ao mantenedor" (context.index), e o que ele apontar vira correção.
- [x] AC-017: a suíte anterior passa com os rótulos novos; portão completo verde. Verificar: teste.
