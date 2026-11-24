# ALI - Agent Local Intelligence

ALI es un espacio de trabajo local con CLI e interfaz de escritorio nativa para preparar briefs desde issues, revisar archivos de texto sensibles, conservar memoria semántica cifrada y crear traspasos compactos entre agentes. La supervisión y el failover automáticos de agentes siguen en la hoja de ruta.

Idiomas: [English](README.md) · [Italiano](README.it.md) · [Français](README.fr.md) · [Español](README.es.md)

## Límites de seguridad actuales

- Los textos de interfaz, la guía CLI y los prompts se cargan desde `config/locales/`, `config/help/`, `config/prompts/`, `config/agent-prompts/` y `config/model-system/`. Si falta una traducción, se produce un error; los diagnósticos técnicos aún no están todos localizados.
- No se aceptan claves API ni tokens de GitHub en argumentos o variables de entorno. El refinamiento solo admite endpoints HTTP loopback. Los embeddings exigen una dirección IP loopback y no siguen redirecciones.
- Los perfiles restringidos no pueden enviar prompts a un modelo. ALI todavía no inicia agentes de programación.
- Los perfiles restringidos también bloquean la copia al portapapeles y la apertura del navegador desde `prepare` y la interfaz. Son controles del flujo elegido, no autenticación ni una frontera de seguridad del sistema operativo.
- El perfil de desarrollo es restringido. ALI no lee ni pasa el entorno del proceso a agentes, que todavía no se ejecutan.
- El análisis y la revisión semántica de documentos son locales. `privacy review` analiza mediante embeddings loopback la copia después de redactar patrones y solo la escribe cuando termina el análisis; no envía el documento a servicios remotos. Solo acepta los formatos de texto UTF-8 configurados.
- La detección es heurística. Un informe limpio no demuestra que el archivo no tenga datos sensibles. Los datos legales, judiciales, sanitarios, financieros y de identidad deben permanecer en local.
- La memoria local cifra textos y vectores con claves derivadas mediante Argon2id y XChaCha20-Poly1305. La frase de contraseña no se guarda. Un proceso del mismo usuario o un dispositivo comprometido aún puede acceder al texto descifrado durante su uso.
- Los traspasos tienen un límite configurado de 50 líneas, se limpian con patrones configurados y se guardan con permisos restrictivos. Revísalos antes de transferirlos a un agente.

Consulta [seguridad](docs/SECURITY.md), [modelo de amenazas](docs/THREAT_MODEL.md), [ciclo de datos](docs/DATA_LIFECYCLE.md) y [reglas para agentes](AGENTS.md).

## Compilación

Rust 1.85 o posterior:

```sh
cargo build --release
```

`Cargo.lock` fija las dependencias. Aún no hay instaladores firmados ni compilaciones reproducibles de publicación.

## Interfaz web local

Inicia la interfaz local del navegador:

```sh
cargo run -- web
```

ALI abre el navegador con una capacidad de un solo uso en un puerto loopback efímero. La sesión caduca a los 30 minutos. Solo procesa issues públicas de GitHub; no acepta documentos, no llama a modelos remotos y no envía prompts.

## Brief de issue

```sh
cargo run -- prepare https://github.com/owner/repository/issues/123 --agent-id codex-cli --copy --open https://chatgpt.com/
cargo run -- --language es prepare https://github.com/owner/repository/issues/123 --output issue-123.prompt.md
```

La descarga usa la API pública de GitHub. La autenticación de repositorios privados no se ha implementado hasta integrar las credenciales con el almacén del sistema. El refinamiento opcional solo usa modelos locales:

```sh
cargo run -- prepare https://github.com/owner/repository/issues/123 \
  --model-endpoint http://127.0.0.1:11434 --model qwen2.5-coder
```

ALI no instala, autentica ni inicia el servidor del modelo. Se rechazan los endpoints fuera de loopback.

## Memoria local

Inicia Ollama y deja disponible el modelo de embeddings configurado. ALI no descarga modelos automáticamente.

```sh
cargo run -- memory init
cat notas.txt | cargo run -- memory add --agent-id codex-cli
printf '%s' 'decisión previa sobre control de acceso' | cargo run -- memory search --agent-id codex-cli
cargo run -- memory list
cargo run -- memory clear
```

La frase de contraseña se solicita sin mostrarla en el terminal. El texto, la etiqueta opcional del agente y los vectores se cifran juntos. Las etiquetas son manuales: ALI no recopila automáticamente actividad de agentes. La búsqueda por agente incluye notas compartidas y las asociadas a ese agente configurado. La solicitud de embedding solo llega a la dirección IP loopback de `config/runtime.toml`. Usa `memory --data-dir /ruta/privada` para cambiar la carpeta; se rechazan symlinks y permisos Unix demasiado amplios.

La GUI nativa también permite inicializar la bóveda, guardar y consultar notas y buscar por agente configurado. Borra los campos de frase y consulta al iniciar la operación y pone a cero los campos sensibles al cerrar la ventana. El borrado de la bóveda y la selección de otro directorio siguen disponibles en la CLI.

## Revisión local de documentos

```sh
cargo run -- --language es privacy scan --input confidencial.txt --output confidencial.review.txt
cargo run -- --language es privacy analyze --input confidencial.txt
cargo run -- --language es privacy review --input confidencial.txt --output confidencial.candidate.txt
```

Los detectores configurados cubren algunos patrones de correo, tokens, claves privadas, código fiscal italiano, SSN estadounidense y tarjetas de pago. `privacy analyze` compara segmentos acotados con descripciones semánticas multilingües configuradas usando el servicio de embeddings loopback. Ambas operaciones mantienen el documento local. Revisa toda la copia y cada aviso: la similitud no es una probabilidad calibrada y no demuestra detección, anonimización, secreto profesional ni autorización para enviar. Nombres, hechos de casos, identificadores indirectos, imágenes, escritura manuscrita y formatos desconocidos pueden pasar inadvertidos. Se rechazan PDF y documentos de Office hasta disponer de analizadores seguros y limitados.

`privacy review` redacta primero los patrones en memoria, analiza semánticamente la copia mediante loopback y solo la escribe cuando termina el análisis. Ningún comando transfiere el documento a un servicio remoto.

## Traspaso de hibernación

Proporciona el esquema JSON documentado por la entrada estándar. Se rechazan campos desconocidos y valores multilínea:

```sh
cat handoff.json | cargo run -- handoff create
```

Campos obligatorios: `session_id`, `failed_agent`, `failure` (`crash`, `timeout`, `non_zero_exit`, `token_limit`, `cancelled` o `unknown`), `verified_progress` (hasta ocho líneas) y `next_action`. El Markdown local se analiza, se limita por líneas y se crea sin sobrescribir archivos. El failover automático de agentes aún no está implementado.

## Configuración e idiomas

| Ruta | Contenido analizado |
|---|---|
| `config/runtime.toml` | Idiomas, perfiles, política de red, embeddings locales, almacenamiento, formatos, KDF y límites. |
| `config/web/{en,it,fr,es}/` | Recursos estáticos localizados servidos solo en la sesión loopback. |
| `config/agents.toml` | Claves de ejecutables y perfiles de prompt de los agentes configurados. No se inician agentes. |
| `config/providers.toml` | API de GitHub y ruta del servicio local compatible. |
| `config/sensitivity.toml` | Detectores locales de patrones sensibles. |
| `config/privacy-review.toml` | Referencias semánticas multilingües, límites de segmentos/lotes/duración y umbral de similitud. |
| `config/help/{en,it,fr,es}.md` | Guía CLI localizada que aparece con `--help`. |
| `config/locales/{en,it,fr,es}.toml` | Mensajes CLI y textos obligatorios de seguridad/consentimiento. |
| `config/prompts/{en,it,fr,es}.md` | Plantillas de briefs localizadas. |
| `config/agent-prompts/{en,it,fr,es}/{agente}.md` | Instrucciones localizadas del agente que se añaden a los briefs generados. |
| `config/model-system/{en,it,fr,es}.md` | Instrucciones localizadas para refinar el modelo. |
| `config/handoffs/{en,it,fr,es}.md` | Plantillas de traspaso localizadas. |

La configuración se carga desde `config/` respecto al directorio actual. `--config-dir` permite seleccionar otra carpeta completa. ALI valida todos los archivos y plantillas de los idiomas antes de iniciar.

## Alcance actual y hoja de ruta

Los comandos disponibles son `prepare`, `memory`, `privacy scan`, `privacy analyze`, `privacy review`, `handoff create`, `agents list`, `gui` y `web`. La interfaz nativa está disponible; la ejecución/supervisión/failover automáticos, perfiles autenticados, análisis PDF/Office e instaladores siguen siendo hitos explícitos. Consulta la [hoja de ruta](docs/ROADMAP.md), [arquitectura](docs/ARCHITECTURE.md), la guía [italiana](README.it.md) y la [francesa](README.fr.md).

## Catálogo de adaptadores

Lista los protocolos configurados con `ali adapters list`. El catálogo se lee desde `config/adapters.toml`; las entradas `planned` son propuestas futuras, no integraciones ejecutables. El cliente de chat compatible con OpenAI y los embeddings de Ollama solo usan loopback. Consulta [interoperabilidad](docs/INTEROPERABILITY.md), [trabajos de entrega](docs/JOBS.md) y la [hoja de ruta](docs/ROADMAP.md).
