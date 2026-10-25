# Serviço de Pagamento v1 (legado)

> Documento fictício da Transportes Serra Azul. O serviço v1 está em modo somente leitura desde a migração para a v2 e será desligado em definitivo em 30/06/2026.

## Visão geral

O serviço de pagamento v1 processa cobranças de frete de clientes corporativos por meio de chamadas síncronas. Cada cobrança gera um registro na tabela `cobrancas_v1` e um comprovante em PDF enviado por e-mail.

## Timeouts e novas tentativas

No serviço de pagamento v1 (legado), o timeout padrão de requisição síncrona era de 120 segundos. Quando o adquirente não respondia dentro desse prazo, o cliente recebia o código ERR_801_GATEWAY e o serviço fazia até 5 novas tentativas, com intervalo fixo de 20 segundos entre elas.

## Limites

- Valor máximo por cobrança: R$ 250.000,00.
- Lote máximo: 200 cobranças por chamada.
- Moedas aceitas: somente real (BRL).

## Notificações

O v1 não tinha webhooks. Os clientes consultavam o status da cobrança por polling no endpoint `/v1/cobrancas/{id}/status`, com no máximo uma consulta por minuto por cobrança.
