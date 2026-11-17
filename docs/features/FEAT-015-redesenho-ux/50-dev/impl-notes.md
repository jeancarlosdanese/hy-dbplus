# @file: docs/features/FEAT-015-redesenho-ux/50-dev/impl-notes.md

# Notas de implementação: FEAT-015

> Protótipo e plano aprovados pelo mantenedor em DD/MM/AAAA.

## Decisões de implementação

- **Tema:** decisão técnica e o motivo.

## Princípios

Decisões de modelo difíceis de mudar, com a data e quem decidiu. Exemplo: "o passado não muda: correção é linha nova".

## Plano

Passos marcáveis, por fase. Cada fase fecha com teste, notas e o "pode commitar" do mantenedor. `⏸️ Mantenedor:` é
passo que só ele executa (migration, reinício de serviço, deploy): vai junto, no começo da fase, com o bloco de
comandos pronto e a saída esperada.

### Fase A: nome

- [ ] ⏸️ A.0 Mantenedor: aplicar as migrations da fase e reiniciar o serviço (comandos abaixo, na ordem).
- [ ] A.1 Passo.
- [ ] A.2 Passo.

## Desvios do plano

<!-- O que mudou durante a fase e por quê. Erro achado na área da FEAT se corrige junto e entra aqui. Decisão tomada
sem perguntar, que o mantenedor possa querer rever, também. Sem desvios: escrever "Sem desvios". -->

## Testes ({{DD/MM/AAAA, ambiente, usuário, dispositivo}})

**Conferido:** {{o que foi testado e como}}

**Não conferido:** {{o que ficou sem teste e o motivo: celular, outro perfil, e-mail real, produção; ou "nada"}}

## Segurança (fechamento)

<!-- Uma linha por rota ou tela nova. Sem rota nem tela nova: escrever "Sem rota nem tela nova". -->

| Rota ou tela | Permissão (mesma chave nos dois lados) | Conta vinda da sessão | Verificado por |
|---|---|---|---|
| {{rota ou tela}} | {{recurso.acao}} | {{sim ou não}} | {{como foi verificado}} |

## Riscos

## Fora desta branch
