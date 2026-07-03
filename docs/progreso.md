# Progreso de Alexandria

## Estado actual
**Versión:** 0.1.0 MVP  
**Fecha:** 2026-07-03  
**Estado:** MVP funcional y validado

## Completado
- [x] Estructura base del proyecto Cargo.
- [x] Documento de visión actualizado (`AlexandriaProject.MD`).
- [x] CLI con `clap`: `scan`, `serve`, `info`.
- [x] Configuración portable (`config.rs`).
- [x] Base de datos SQLite con migraciones SQLx.
- [x] Escáner recursivo con concurrencia limitada.
- [x] Extractor de metadatos del sistema de archivos.
- [x] Extractor ffprobe opcional con fallback.
- [x] Servidor Axum con API REST y frontend embebido.
- [x] Sistema de logs con `tracing` y `tracing-appender`.
- [x] Tests unitarios e integración pasando.
- [x] Validación E2E: scan, info, serve, API y frontend.
- [x] README, .gitignore, LICENSE, run.bat y docs.

## Pendientes para siguiente versión
- [ ] Agrupación de series y películas por patrones.
- [ ] Soporte para más formatos (PDF, ZIP, audio).
- [ ] Mejoras de UI/UX en el frontend.
- [ ] Pruebas de carga con discos grandes.

## Métricas
- Líneas de código fuente: ~2.500 (aproximado).
- Tests: 7 (2 unitarios + 5 integración).
- Build release: ~1m 30s en este entorno.
- Tamaño binario release: ~6.6 MB.
