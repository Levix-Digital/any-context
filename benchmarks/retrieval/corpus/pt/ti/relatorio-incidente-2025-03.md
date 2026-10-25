# Relatório de Incidente — Indisponibilidade do WMS (março de 2025)

> Documento fictício da Transportes Serra Azul.

## Resumo

Em 12/03/2025, o WMS ficou indisponível por 4 horas e 10 minutos, das 06:20 às 10:30. Nenhuma separação de pedidos foi feita no período nos centros de distribuição de Campinas e Contagem.

## Causa raiz

O certificado TLS do balanceador de carga expirou às 06:18. A renovação automática estava desativada desde uma mudança de configuração feita em janeiro.

## Ações corretivas

1. Renovação automática reativada e monitorada.
2. Alerta de vencimento de certificado com 30 dias de antecedência.
3. Revisão de todas as mudanças de configuração de janeiro.
