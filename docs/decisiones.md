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

## [2026-07-03] Notas históricas + nota principal
- **Contexto:** El esquema inicial ya tenía tabla `notes` pero el endpoint actualizaba solo `files.notes`.
- **Opciones:**
  - A: Migrar todo a tabla `notes` y eliminar `files.notes`.
  - B: Mantener `files.notes` como nota principal rápida y tabla `notes` como historial.
- **Decision:** B.
- **Justificación:** Conserva la simplicidad del campo `notes` para lecturas frecuentes (listados, detalle rápido) mientras se guarda un historial inmutable de cambios. El endpoint `POST /api/files/:id/notes` actualiza ambas.

## [2026-07-03] Tags como tabla independiente
- **Contexto:** Permitir etiquetar archivos de forma libre.
- **Opciones:**
  - A: Campo `tags` en `files` como texto plano separado por comas.
  - B: Tablas normalizadas `tags` y `file_tags` con relación N:M.
- **Decision:** B.
- **Justificación:** Facilita búsquedas, evita duplicados de nombres, permite renombrar etiquetas globalmente y es más escalable para futuras relaciones (grupos, filtros, etc.).

## [2026-07-03] Filtros y ordenamiento dinámicos en SQL
- **Contexto:** La UI requiere ordenar y filtrar por fecha de modificación.
- **Opciones:**
  - A: Ordenar y filtrar en memoria después de traer datos.
  - B: Construir SQL dinámico con `match` para columnas/orden permitidos.
- **Decision:** B.
- **Justificación:** Eficiencia en grandes volúmenes; el `match` en Rust sobre valores fijos evita inyección SQL sin necesidad de ORM.

## [2026-07-04] Reorganización física con dry-run + log + rollback
- **Contexto:** Necesitamos mover/renombrar archivos físicamente según criterios del usuario.
- **Opciones:**
  - A: Ejecutar movimientos directamente sin registro previo.
  - B: Generar un plan persistente (`reorg_jobs` + `reorg_operations`), permitir dry-run, aplicar paso a paso y poder revertir.
- **Decision:** B.
- **Justificación:** Minimiza el riesgo de pérdida de datos: se puede previsualizar, se registra cada operación, se hace backup de la BD, se verifican checksums en copias y se dispone de rollback. Además, `std::fs::rename` dentro del mismo volumen es atómico, lo que reduce la ventana de inconsistencia.

## [2026-07-04] Blake3 para verificación de integridad
- **Contexto:** Verificar que una copia entre volúmenes no se corrompió antes de borrar el original.
- **Opciones:** SHA-256 vs Blake3.
- **Decision:** Blake3.
- **Justificación:** Blake3 es significativamente más rápido en un único hilo para grandes archivos y tiene una implementación Rust madura y sencilla.

## [2026-07-04] Cross-volume como opt-in
- **Contexto:** Los movimientos entre discos implican copia + borrado, con más riesgo y tiempo.
- **Opciones:**
  - A: Permitir cross-volume por defecto.
  - B: Requerir flag explícita `--allow-cross-volume`.
- **Decision:** B.
- **Justificación:** Por defecto se usan renombrados atómicos (mismo volumen), más seguros e instantáneos. El usuario debe optar conscientemente por operaciones entre discos.
