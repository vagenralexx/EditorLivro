# ADR-001 — Desktop local-first

**Status:** proposta inicial  
**Data:** 2026-10-11

## Contexto

O produto precisa editar arquivos locais, criar backups, abrir pacotes EPUB, reempacotá-los com validação e manter chaves de IA privadas. O protótipo atual é uma página web e depende de permissões que variam por navegador/contexto.

## Decisão proposta

Priorizar uma aplicação desktop local-first. Avaliar Tauri como primeira implementação e Electron como alternativa a comparar no spike técnico. O núcleo deve depender de portas de filesystem/ZIP/secure storage, evitando acoplamento do domínio à tecnologia de shell.

## Consequências positivas

- Gravação e backup confiáveis em arquivos reais.
- Chaves em armazenamento seguro do sistema.
- Workspace EPUB, validação e ferramentas locais sem depender de permissões do navegador.
- Privacidade por padrão: o texto não sai da máquina exceto em chamadas IA consentidas.

## Consequências/custos

- Há empacotamento, atualização e testes por sistema operacional.
- É necessário manter limites de segurança entre UI web e comandos nativos.
- A escolha Tauri/Electron precisa de prova com EPUBs reais antes de compromisso definitivo.

## Critério para aceitar a decisão

O spike deve abrir uma pasta XHTML, criar/restaurar backup, importar/exportar um EPUB válido e armazenar uma credencial de teste sem expô-la ao renderer.

