const pageSize = 25;
let currentOffset = 0;
let currentFilters = {};
let selectedFileId = null;

function formatBytes(bytes) {
    if (bytes === 0 || bytes == null) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatDuration(seconds) {
    if (seconds == null) return '-';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return `${h}h ${m}m ${s}s`;
}

async function loadStats() {
    try {
        const res = await fetch('/api/stats');
        const stats = await res.json();
        document.getElementById('total-files').textContent = stats.total_files ?? 0;
        document.getElementById('video-files').textContent = stats.video_files ?? 0;
        document.getElementById('total-size').textContent = formatBytes(stats.total_size_bytes ?? 0);
        document.getElementById('last-scan').textContent = stats.last_scan
            ? new Date(stats.last_scan).toLocaleString()
            : 'Nunca';
    } catch (err) {
        console.error('Error loading stats:', err);
    }
}

async function loadFiles() {
    const params = new URLSearchParams();
    params.set('limit', pageSize);
    params.set('offset', currentOffset);
    if (currentFilters.name) params.set('name', currentFilters.name);
    if (currentFilters.extension) params.set('extension', currentFilters.extension);
    if (currentFilters.min_size) params.set('min_size', currentFilters.min_size);
    if (currentFilters.max_size) params.set('max_size', currentFilters.max_size);

    try {
        const res = await fetch('/api/files?' + params.toString());
        const data = await res.json();
        renderFiles(data.data || []);
        document.getElementById('page-info').textContent = `Offset ${currentOffset}`;
    } catch (err) {
        console.error('Error loading files:', err);
    }
}

function renderFiles(files) {
    const tbody = document.querySelector('#files-table tbody');
    tbody.innerHTML = '';
    for (const f of files) {
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td><a href="#" data-id="${f.id}">${escapeHtml(f.name)}</a></td>
            <td>${f.extension || '-'}</td>
            <td>${formatBytes(f.size_bytes)}</td>
            <td>${formatDuration(f.duration_seconds)}</td>
            <td>${f.width && f.height ? f.width + 'x' + f.height : '-'}</td>
            <td>${f.has_subtitles ? 'Sí' : 'No'}</td>
        `;
        tr.querySelector('a').addEventListener('click', (e) => {
            e.preventDefault();
            showDetail(f.id);
        });
        tbody.appendChild(tr);
    }
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

async function showDetail(id) {
    selectedFileId = id;
    try {
        const res = await fetch('/api/files/' + id);
        if (!res.ok) throw new Error('File not found');
        const payload = await res.json();
        const f = payload.data;
        const content = document.getElementById('detail-content');
        content.innerHTML = `
            <p><strong>Ruta:</strong> ${escapeHtml(f.path)}</p>
            <p><strong>Tamaño:</strong> ${formatBytes(f.size_bytes)}</p>
            <p><strong>Modificado:</strong> ${new Date(f.modified_at).toLocaleString()}</p>
            <p><strong>Duración:</strong> ${formatDuration(f.duration_seconds)}</p>
            <p><strong>Resolución:</strong> ${f.width && f.height ? f.width + 'x' + f.height : '-'}</p>
            <p><strong>Vídeo:</strong> ${f.video_codec || '-'}</p>
            <p><strong>Audio:</strong> ${f.audio_codec || '-'}</p>
            <p><strong>Pistas de audio:</strong> ${f.audio_tracks || '-'}</p>
            <p><strong>Subtítulos:</strong> ${f.subtitle_tracks || '-'}</p>
        `;
        document.getElementById('note-input').value = f.notes || '';
        document.getElementById('detail').classList.remove('hidden');
    } catch (err) {
        console.error('Error loading detail:', err);
    }
}

async function saveNote() {
    if (selectedFileId == null) return;
    const content = document.getElementById('note-input').value;
    try {
        const res = await fetch('/api/files/' + selectedFileId + '/notes', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ content })
        });
        if (!res.ok) throw new Error('Failed to save note');
        alert('Nota guardada');
    } catch (err) {
        console.error('Error saving note:', err);
        alert('Error al guardar la nota');
    }
}

function readFilters() {
    currentFilters = {
        name: document.getElementById('filter-name').value || null,
        extension: document.getElementById('filter-ext').value || null,
        min_size: document.getElementById('filter-min').value || null,
        max_size: document.getElementById('filter-max').value || null,
    };
    currentOffset = 0;
}

document.getElementById('btn-search').addEventListener('click', () => {
    readFilters();
    loadFiles();
});

document.getElementById('btn-prev').addEventListener('click', () => {
    if (currentOffset >= pageSize) {
        currentOffset -= pageSize;
        loadFiles();
    }
});

document.getElementById('btn-next').addEventListener('click', () => {
    currentOffset += pageSize;
    loadFiles();
});

document.getElementById('btn-save-note').addEventListener('click', saveNote);

loadStats();
loadFiles();
