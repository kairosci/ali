# ALI - guía de comandos

ALI mantiene localmente la revisión de documentos, la memoria y el análisis de modelos por defecto. Esta guía se carga desde el directorio de configuración seleccionado.

## Opciones globales

- `--config-dir RUTA`: selecciona un directorio de configuración completo. Predeterminado: `config`; variable: `ALI_CONFIG_DIR`.
- `--language en|it|fr|es`: selecciona el idioma de la interfaz y los prompts. Predeterminado: `product.default_locale` en `runtime.toml`.
- Sin comando, con `--help`, `-h` o `help`: muestra esta guía configurada.

## Comandos

- `ali prepare URL [--output RUTA] [--copy] [--open URL] [--model-endpoint URL_LOOPBACK --model NOMBRE] [--profile PERFIL] [--agent-id ID]`: obtiene por HTTPS una issue pública de GitHub y crea un brief local. El refinamiento es opcional, solo local y está desactivado para perfiles restringidos. Abrir un sitio no pega el brief.
- `ali memory init|add|search|list|clear [--data-dir RUTA]`: gestiona el almacén local cifrado. `add` y `search` leen de la entrada estándar; `--agent-id ID` etiqueta una nota o busca notas compartidas y las asociadas al agente configurado. Ollama debe estar iniciado en el endpoint loopback configurado para embeddings.
- `ali privacy scan --input RUTA --output RUTA`: analiza localmente un archivo de texto UTF-8 configurado y crea una copia redactada independiente.
- `ali privacy analyze --input RUTA`: compara segmentos acotados con descripciones semánticas multilingües configuradas usando el servicio de embeddings loopback. Las señales no autorizan la divulgación.
- `ali privacy review --input RUTA --output RUTA`: redacta localmente los patrones configurados, revisa semánticamente la copia mediante embeddings loopback y solo la escribe si termina el análisis local. Nunca transfiere el documento a un servicio remoto.
- `ali handoff create [--output RUTA]`: lee de la entrada estándar el esquema JSON validado y crea una nota local revisada de hasta 50 líneas.
- `ali agents list`: inspecciona solo las rutas de comandos declaradas en `config/agents.toml`; no inicia agentes.
- `ali web`: abre la interfaz local del navegador en loopback. Prepara prompts desde issues públicas de GitHub y no acepta documentos.
- `ali gui`: abre la interfaz de escritorio nativa.

## Límite de datos

Esta versión no tiene cuentas ni inicia agentes. Obtener una issue pública es una solicitud explícita a la red de GitHub. El análisis de documentos, la revisión semántica y los embeddings de memoria son locales; los endpoints de embeddings deben usar una IP loopback. La revisión semántica es una ayuda incompleta: no garantiza detección, anonimato, secreto profesional ni autorización para divulgar. Mantén local la información restringida y revisa todos los resultados.

### adapters list

Muestra los protocolos de modelos configurados y su estado de implementación. Las entradas planificadas todavía no se pueden usar.
