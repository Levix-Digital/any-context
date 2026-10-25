# Serviço de Pagamento v2

> Documento fictício da Transportes Serra Azul. Versão atual do serviço de pagamento, em produção desde 02/02/2026.

## Visão geral

O serviço de pagamento v2 substitui o v1 e passa a aceitar cobranças assíncronas, Pix e cartão corporativo. As cobranças ficam na tabela `cobrancas` e o comprovante é gerado sob demanda.

## Timeouts e novas tentativas

No serviço de pagamento v2, o timeout padrão de requisições síncronas é de 30 segundos e retorna ERR_902_TIMEOUT quando estoura. Requisições assíncronas não têm timeout para o cliente: a resposta chega pelo webhook de status. O v2 faz até 3 novas tentativas no adquirente, com backoff exponencial começando em 2 segundos.

## Limites

- Valor máximo por cobrança: R$ 500.000,00.
- Lote máximo: 1.000 cobranças por chamada assíncrona.
- Moedas aceitas: real (BRL) e dólar americano (USD) para clientes com contrato internacional.

## Idempotência

Toda chamada de criação deve enviar o cabeçalho `Idempotency-Key`. Chaves repetidas dentro de 24 horas devolvem a cobrança já criada, sem nova cobrança.
