# @file: docs/features/FEAT-015-redesenho-ux/10-prd/acceptance.md

# Critérios de aceite: FEAT-015

Cenários em Gherkin; os exemplos numéricos do brief viram casos. Inclua o que **não** pode acontecer: permissão
recusada, dado de outra conta, módulo desligado. Critério de aceite é o que um teste prova antes do deploy. Texto
entre chaves duplas é do template: o verificador avisa enquanto ele estiver aqui.

```gherkin
Cenário: {{caminho feliz}}
  Dado {{um usuário com a permissão}}
  Quando {{faz a ação}}
  Então {{o resultado esperado aparece}}

Cenário: {{usuário sem permissão}}
  Dado {{um usuário sem a permissão}}
  Quando {{tenta a mesma ação}}
  Então {{a rota é recusada}}
  E {{a resposta do servidor não traz o dado}}
```

## Checklist

Um item por critério, com a forma de verificar: `teste` ou `navegador no dev`. Critério que ficar aberto leva
`motivo:` ao lado.

- [ ] AC-001: {{o que deve valer}}. Verificar: teste.
- [ ] AC-002: {{o que deve valer}}. Verificar: navegador no dev.
