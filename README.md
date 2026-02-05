# MISES

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-MIT)
![Test Status](https://github.com/aicacia/rs-mises/actions/workflows/test.yml/badge.svg)

Mesh Identity and Security Enforcement System

```bash
grpcurl -plaintext -authority dummy unix://${PWD}/mises.sock list
grpcurl -plaintext -authority dummy unix://${PWD}/mises.sock mises.BootstrapService/Bootstrap
```
