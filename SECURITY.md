# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |

## Reporting a Vulnerability

Please report security vulnerabilities by emailing security@yourdomain.com.

We will acknowledge receipt of your vulnerability report and send you regular updates about our progress.
If you have not received a response within 48 hours, please contact us again to make sure we received your report.

## Security Measures

### Authentication & Authorization
- All API endpoints require authentication via API keys
- Hierarchical API key system with master key support
- API keys have configurable expiration
- Rate limiting per API key

### Data Security
- Data is encrypted in transit using TLS
- PostGIS data is validated and sanitized
- Input SRID validation and transformation
- Secure geometry handling
- Rate limiting per API key
- API key expiration
- Automatic cleanup of expired keys
- Secure backup storage with encryption

### Infrastructure Security
- OpenShift Security Context Constraints (SCC)
- Network policies for pod isolation
- Non-root container execution
- Read-only root filesystem
- Dropped container capabilities

### Monitoring & Auditing
- Regular security audits are performed
- Dependencies are regularly updated and monitored for vulnerabilities
- Request logging with timing information
- Prometheus metrics for monitoring
- Health and readiness checks

### Backup & Recovery
- Automated daily backups
- Configurable backup retention
- Secure backup storage

### Development Practices
- Automated vulnerability scanning in CI/CD
- Regular dependency updates
- Code review requirements
- Security-focused test suite

## Security Contacts

- Security Team: security@yourdomain.com
- Operations Team: ops@yourdomain.com
- Emergency Contact: emergency@yourdomain.com

Response times:
- Critical vulnerabilities: < 24 hours
- High severity: < 48 hours
- Medium/Low severity: < 1 week 