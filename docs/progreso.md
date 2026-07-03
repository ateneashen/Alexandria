# Progreso de Alexandria

## Estado actual
**Versión:** 0.2.0 (Fase 1 completada)  
**Fecha:** 2026-07-03  
**Estado:** Agrupación inteligente implementada y validada

## Completado
- [x] Estructura base del proyecto Cargo.
- [x] Documento de visión actualizado (`AlexandriaProject.MD`).
- [x] CLI con `clap`: `scan`, `serve`, `info`, `groups`, `regroup`.
- [x] Configuración portable (`config.rs`).
- [x] Base de datos SQLite con migraciones SQLx.
- [x] Escáner recursivo con concurrencia limitada.
- [x] Extractor de metadatos del sistema de archivos.
- [x] Extractor ffprobe opcional con fallback.
- [x] Servidor Axum con API REST y frontend vanilla embebido.
- [x] Sistema de logs con `tracing` y `tracing-appender`.
- [x] **Agrupación inteligente**: series (S01E02), películas (año + versión) y colecciones por prefijo.
- [x] Tabla `groups`, columna `group_id` en `files`, endpoints `/api/groups`.
- [x] Tests unitarios e integración pasando.
- [x] Validación E2E: scan, info, groups, serve, API y frontend.
- [x] README, .gitignore, LICENSE, run.bat y docs.
- [x] Repo subido a GitHub.

## Pendientes
- [ ] Fase 2: Soporte para más formatos (PDF, ZIP, audio).
- [ ] Fase 3: Mejora de interfaz web (dashboard, grupos, filtros avanzados).

## Métricas
- Líneas de código fuente: ~3.000 (aproximado).
- Tests: 11 (5 unitarios + 6 integración).
- Build release: ~1m 45s en este entorno.
- Tamaño binario release: ~6.6 MB.
