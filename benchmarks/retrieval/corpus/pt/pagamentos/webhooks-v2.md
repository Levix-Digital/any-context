# Webhooks do Serviço de Pagamento v2

> Documento fictício da Transportes Serra Azul.

## Entrega

Para webhooks do ecossistema v2, o timeout de conexão é de 5 segundos por tentativa. Uma entrega que falha é repetida até 6 vezes, com backoff exponencial (1, 2, 4, 8, 16 e 32 minutos). Depois da sexta falha o evento vai para a fila de eventos mortos, onde fica guardado por 14 dias.

## Assinatura

Cada requisição de webhook leva o cabeçalho `X-Serra-Signature`, uma assinatura HMAC-SHA256 do corpo com o segredo do cliente. O cliente deve rejeitar eventos com mais de 5 minutos de diferença entre o cabeçalho `X-Serra-Timestamp` e o relógio do servidor.

## Eventos

- `cobranca.criada`
- `cobranca.paga`
- `cobranca.estornada`
- `cobranca.expirada` (Pix não pago em 30 minutos)
