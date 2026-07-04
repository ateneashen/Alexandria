# Progreso de Alexandria

## Estado actual
**Versión:** 0.4.0 (Fase de Reorganización completada)  
**Fecha:** 2026-07-04  
**Estado:** Reorganización física de archivos implementada con énfasis en seguridad; beta copiada a `Z:\AlexandriaProjectBeta`

## Completado
- [x] Estructura base del proyecto Cargo.
- [x] Documento de visión actualizado (`AlexandriaProject.MD`).
- [x] CLI con `clap`: `scan`, `serve`, `info`, `groups`, `regroup`, `note`, `tag`, `reorg`.
- [x] Configuración portable (`config.rs`).
- [x] Base de datos SQLite con migraciones SQLx.
- [x] Escáner recursivo con concurrencia limitada.
- [x] Extractor de metadatos del sistema de archivos.
- [x] Extractor ffprobe opcional con fallback (video y audio).
- [x] Extractor de PDF (`lopdf`): páginas, título, autor, etc.
- [x] Extractor de ZIP: conteo de archivos y listado parcial de contenido.
- [x] Servidor Axum con API REST y frontend vanilla embebido.
- [x] Sistema de logs con `tracing` y `tracing-appender`.
- [x] **Agrupación inteligente**: series (S01E02), películas (año + versión) y colecciones por prefijo.
- [x] Tabla `groups`, columna `group_id` en `files`, endpoints `/api/groups`.
- [x] **Fase 3: UI/UX mejorada**:
  - [x] Navegación por pestañas (Dashboard / Archivos / Grupos / Reorganizar).
  - [x] Dashboard con estadísticas y breakdown por tipo de archivo.
  - [x] Listado de archivos con filtros avanzados y paginación con total.
  - [x] Listado de grupos con detalle de archivos.
  - [x] Panel de detalle con metadatos y `extra_json` formateado.
- [x] **Fase 4: Notas y user tags**:
  - [x] Tablas `notes`, `tags` y `file_tags`.
  - [x] Endpoints REST para notas históricas y etiquetas.
  - [x] Sección de etiquetas y historial de notas en el detalle de archivo.
- [x] **Fase 5: Refinamiento de API y UI**:
  - [x] Filtros de fecha y ordenamiento en `/api/files`.
  - [x] Endpoints auxiliares: `/api/files/count`, `/api/file-types`, `/api/extensions`, `/api/scan-jobs`.
  - [x] Stats extendidas con conteos por tipo de archivo.
  - [x] CLI `note` y `tag`.
- [x] **Fase de Reorganización Física (v0.4.0)**:
  - [x] Tablas `reorg_jobs` y `reorg_operations`.
  - [x] Motor de plantillas con tokens (`{file_type}`, `{group_name}`, `{year}`, `{tag}`, etc.).
  - [x] Planificador con detección de colisiones y validación de destinos peligrosos.
  - [x] Ejecutor con movimientos atómicos (mismo volumen) y copy+verify (cross-volume opt-in).
  - [x] Backup automático de la BD antes de aplicar.
  - [x] Verificación de integridad con Blake3 para copias.
  - [x] Rollback de operaciones completadas.
  - [x] CLI `reorg plan/list/status/apply/rollback` con confirmación interactiva.
  - [x] API REST `/api/reorganize/*`.
  - [x] Pestaña "Reorganizar" en el frontend.
  - [x] Tests de reorganización (plan, apply, rollback, colisión).
- [x] Beta funcional copiada a `Z:\AlexandriaProjectBeta` con `README-BETA.md`.
- [x] Tests unitarios e integración pasando.
- [x] Validación E2E: scan, info, groups, serve, API, frontend y reorg.
- [x] README, .gitignore, LICENSE, run.bat y docs.
- [x] Repo subido a GitHub.

## Pendientes
- [ ] Fase 6: Soporte para más formatos (RAR, 7z, imágenes, etc.) y pruebas de carga.
- [ ] Empaquetado/distribución (instalador o release automático).

## Métricas
- Líneas de código fuente: ~4.800 (aproximado).
- Tests: 22 (10 unitarios + 12 integración).
- Build release: ~2m 10s en este entorno.
- Tamaño binario release: ~9.5 MB.
