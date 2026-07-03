# Decisiones de Arquitectura

## [2026-07-03] Axum en lugar de Actix-web
- **Contexto:** El documento original mencionaba Actix-web o Axum.
- **Opciones:** Actix-web (más maduro, más dependencias) vs Axum (más ligero, integrado con Tokio/Tower).
- **Decision:** Axum.
- **Justificación:** Menor superficie de dependencias, mejor integración con el ecosistema Tokio, y suficiente para un MVP de API REST simple.

## [2026-07-03] SQLx con SQLite embebido
- **Contexto:** Necesitamos una base de datos local sin instalación externa.
- **Opciones:** rusqlite (sincrónico) vs sqlx (asincrónico, migraciones integradas).
- **Decision:** sqlx con feature `sqlite`.
- **Justificación:** Permite async nativo con Tokio, incluye migraciones, y SQLite se compila embebido sin requerir bibliotecas del sistema.

## [2026-07-03] ffprobe opcional
- **Contexto:** Extraer metadatos profundos de video.
- **Opciones:** Librerías puras Rust (inmaduras o inexistentes para todos los formatos) vs ejecutar ffprobe como proceso hijo.
- **Decision:** Llamar a ffprobe si está disponible; fallback a metadatos del filesystem.
- **Justificación:** ffprobe es el estándar de la industria, robusto y soporta casi cualquier formato. Hacerlo opcional mantiene la app portable incluso sin él.

## [2026-07-03] Frontend vanilla embebido
- **Contexto:** Interfaz web ligera.
- **Opciones:** React/Vue/Svelte vs vanilla JS.
- **Decision:** Vanilla JS/CSS/HTML embebido con `include_str!`.
- **Justificación:** Cero dependencias de build, frontend se incluye en el binario, ideal para portabilidad.

## [2026-07-03] Diseño del sistema de grupos
- **Contexto:** Necesitamos detectar series, películas duplicadas/versiones y colecciones por nombre.
- **Opciones:**
  - A: Algoritmo puramente basado en distancia de strings (Levenshtein).
  - B: Reglas basadas en expresiones regulares para patrones conocidos (S01E02, años, etc.) más fallback por prefijo.
- **Decision:** B.
- **Justificación:** Es más predecible, explicable y rápido para el usuario. Detecta series y películas con alta confianza sin requerir entrenamiento ni umbrales mágicos. El fallback por prefijo cubre casos generales.

## [2026-07-03] Extracción de PDF con `lopdf`
- **Contexto:** Ampliar soporte a documentos PDF.
- **Opciones:** `lopdf` (puramente Rust, parsea la estructura PDF) vs herramientas externas como `pdfinfo`.
- **Decision:** `lopdf` con el parser `nom` y sin features por defecto.
- **Justificación:** No requiere binarios externos, se integra en el binario final y es suficiente para leer el diccionario `Info` y contar páginas. Se deshabilitaron features innecesarias para reducir dependencias.

## [2026-07-03] Extracción de ZIP con `zip`
- **Contexto:** Listar contenido de archivos comprimidos.
- **Opciones:** `zip` (librería Rust madura para .zip) vs ejecutar `unzip -l`.
- **Decision:** `zip` con compresión `deflate` y `Stored`.
- **Justificación:** Procesamiento local, sin dependencia de utilidades del sistema operativo. Permite obtener conteo de entradas y nombres de archivos de forma rápida.

## [2026-07-03] Metadatos de audio vía `ffprobe`
- **Contexto:** Reutilizar la infraestructura de extracción para audio.
- **Opciones:** Librerías puras Rust para MP3/FLAC (complejas y con soporte parcial) vs reutilizar ffprobe.
- **Decision:** Reutilizar `ffprobe` para audio, usando la misma salida JSON que para video.
- **Justificación:** Unifica el código de análisis multimedia, aprovecha el soporte de múltiples codecs y tags, y mantiene la herramienta opcional.
