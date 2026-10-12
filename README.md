# Editor de Livros EPUB

Aplicativo desktop local-first para editar XHTML e EPUB com prévia de leitura e assistência editorial por IA.

## Iniciar em desenvolvimento

```powershell
npm run dev
```

## Verificar frontend

```powershell
npm run build
```

## Base implementada

- `apps/desktop`: Tauri + React, janela desktop e interface inicial de leitor/editor.
- `packages/editorial-rules`: contrato inicial que transforma a skill em pedidos, sugestões e resultados editoriais estruturados.
- `packages/ai`: contrato agnóstico de provedor, com padrão inicial para NVIDIA NIM e espaço para Groq/OpenAI.
- Demais packages: fronteiras documentadas para EPUB, XHTML, backup, PDF, leitura, validação e persistência.
- `vault-editor-epub`: decisões de produto, arquitetura, bibliotecas e roadmap.

## Próxima entrega

Implementar `FileStore` seguro + seletor de pasta/EPUB, seguido por importação XHTML com snapshot de backup antes da gravação.
