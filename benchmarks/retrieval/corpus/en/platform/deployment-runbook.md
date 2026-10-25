# Production Deployment Runbook

> Fictitious document of Kestrel Analytics.

## Strategy

Production deploys use blue/green: the new version is started next to the old one and traffic moves only after the health checks pass for 10 minutes.

## Rollback

If the error rate goes above 2% after the switch, the on-call engineer rolls back to the previous color. A rollback must be completed within 15 minutes of the alert.

## Change freeze

There are no production deploys on Fridays after 14:00 UTC, on weekends, or during the last two business days of a quarter. Emergency fixes need approval from the head of engineering.
