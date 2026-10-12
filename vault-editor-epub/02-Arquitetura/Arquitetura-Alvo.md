# Arquitetura-alvo

## Recomendação

Construir um aplicativo desktop **local-first**, com shell nativo (Tauri é a primeira opção a avaliar) e frontend web tipado. O motivo não é estética: EPUB, backups e chaves de API precisam de acesso confiável ao disco e armazenamento seguro, algo que uma página HTML isolada não entrega de forma consistente.

Electron continua uma alternativa se a equipe preferir ecossistema Node/TypeScript; a decisão final deve vir de um spike curto com importação e exportação real de EPUB. A estrutura de módulos já está definida em [[Estrutura-do-Repositorio]] e as escolhas de dependências ficam deliberadamente isoladas em [[Bibliotecas-e-Spikes]].

## Componentes

```text
UI (biblioteca, editor, leitor, painel IA, histórico)
                 │ comandos tipados
Aplicação / casos de uso
  ├─ Projeto de pasta XHTML
  ├─ Projeto EPUB
  ├─ Backup e restauração
  ├─ Validação e exportação
  └─ Orquestração de IA
                 │ portas
Infraestrutura local
  ├─ sistema de arquivos + atomic write
  ├─ ZIP/EPUB + EPUBCheck
  ├─ banco local de projeto/histórico
  ├─ keychain/secure store
  └─ clientes OpenAI-compatible / provedores
```

## Modelo de domínio mínimo

- `BookProject`: tipo (`folder` ou `epub`), origem, workspace, configuração e estado.
- `Publication`: metadados, pacote OPF, manifest, spine, navigation e recursos.
- `Document`: caminho interno, XHTML original, árvore/versão atual, estado de validação e histórico.
- `ChangeSet`: documento, operações, origem (`manual`, `ia`, `bulk`), autor, data e diff.
- `BackupSnapshot`: hash, local, motivo, data e conjunto de arquivos.
- `EditorialProfile`: idioma, gênero, tom, glossário, regras ativas e limiares.
- `AiProviderConfig`: provedor, endpoint, modelo, política de privacidade e referência à chave no cofre.

## Fluxo para pasta XHTML

1. Selecionar pasta e criar um projeto local apontando para ela.
2. Indexar XHTML/HTML, CSS e imagens sem mover nada.
3. Abrir documento no editor transacional; a prévia recebe versão sanitizada em sandbox.
4. Ao salvar: validar → mostrar resumo de alteração → criar snapshot → escrever atomicamente → registrar versão.

## Fluxo para EPUB

1. Copiar/importar o EPUB para uma área de trabalho controlada; nunca editar o ZIP de origem durante a sessão.
2. Ler `META-INF/container.xml`, localizar o OPF, montar manifest/spine/nav e disponibilizar somente documentos do spine para edição normal.
3. Manter caminhos e recursos originais; alterações ocorrem no workspace extraído.
4. Exportar para novo arquivo por padrão; substituir o EPUB original é uma ação separada e confirmada, com backup.

## Editor e leitor

- **Editor fonte:** XHTML com realce, validação XML e formatação que respeite o esquema.
- **Editor visual:** deve operar sobre uma árvore estruturada e gerar operações/diff, não reserializar HTML arbitrário.
- **Leitor:** iframe sandboxed, CSS isolado, CSS de dispositivo/preferências sobreposto sem alterar o arquivo salvo.
- **Mapeamento:** seleção no leitor e no código deve apontar para o mesmo nó/trecho para permitir sugestões IA ancoradas.

## Contratos entre módulos

Nenhum módulo de interface deve chamar ZIP, filesystem, keychain ou API de IA diretamente. Cada integração atravessa uma porta tipada, por exemplo `FileStore`, `ArchiveStore`, `SecureSecretStore`, `PdfRenderer`, `EpubValidator` e `LanguageModelGateway`. Assim, Tauri/Electron, EPUBCheck/Vivliostyle e os provedores de IA podem ser trocados sem reescrever os casos de uso.

## Dados locais

Guardar no diretório de projeto ou em diretório de dados do app: índice, preferências, histórico, relatórios e referências a backups. Não guardar a chave de API no arquivo do projeto ou em Markdown; usar o armazenamento seguro do sistema operacional.

