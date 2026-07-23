/**
 * FlowCut — Timeline Controller
 * Manages the timeline canvas rendering, interaction, and state.
 * Handles playhead movement, clip operations, and track management.
 */

class FlowCutTimeline {
    constructor() {
        this.canvas = document.getElementById('timeline-canvas');
        this.ctx = this.canvas.getContext('2d');
        this.scrollArea = document.getElementById('timeline-scroll');
        this.trackHeaders = document.getElementById('track-headers');

        this.trackManager = new TrackManager();
        this.clipRenderer = new ClipRenderer(this.ctx);
        this.selectedClipId = null;
        this.selectedTrackId = null;

        this.playheadTime = 0;
        this.pixelsPerSecond = 50; // zoom level
        this.snapEnabled = true;
        this.snapThreshold = 10; // pixels

        this.isDragging = false;
        this.dragType = null; // 'move', 'trimLeft', 'trimRight'
        this.dragClipId = null;
        this.dragStartX = 0;
        this.dragStartTime = 0;

        this.interactionMode = 'select'; // 'select', 'razor', 'slip', 'slide', 'ripple'

        this.initCanvas();
        this.initDefaultTracks();
        this.bindEvents();
    }

    /**
     * Initialize canvas dimensions and rendering loop.
     */
    initCanvas() {
        this.resizeCanvas();
        window.addEventListener('resize', () => this.resizeCanvas());
        this.renderLoop();
    }

    resizeCanvas() {
        if (!this.canvas) return;
        const container = this.scrollArea;
        this.canvas.width = container?.clientWidth || 800;
        this.canvas.height = (this.trackManager.tracks.length * 48) + 24; // time ruler height
    }

    /**
     * Create default video and audio tracks.
     */
    initDefaultTracks() {
        this.trackManager.addTrack('Video', 'Video 1');
        this.trackManager.addTrack('Audio', 'Audio 1');
        this.renderTrackHeaders();
        this.resizeCanvas();
    }

    /**
     * Render the track headers sidebar.
     */
    renderTrackHeaders() {
        if (!this.trackHeaders) return;
        this.trackHeaders.innerHTML = '';
        this.trackManager.tracks.forEach(track => {
            const header = document.createElement('div');
            header.className = `track-header${track.locked ? ' locked' : ''}${!track.visible ? ' hidden-track' : ''}`;
            header.dataset.track = track.id;

            const typeColor = track.type === 'Video' ? '' : ' audio';
            header.innerHTML = `
                <span class="track-label${typeColor}">${track.id}</span>
                <span class="track-name">${track.name}</span>
                <div class="track-controls">
                    <button class="track-btn ${track.locked ? 'active' : ''}" data-action="lock-track" title="Lock Track">
                        <svg viewBox="0 0 16 16" width="14" height="14"><path d="M4 7V5C4 3 5 2 8 2S12 3 12 5V7" stroke="currentColor" stroke-width="1.5" fill="none"/><rect x="3" y="7" width="10" height="7" rx="1" fill="currentColor"/></svg>
                    </button>
                    <button class="track-btn ${!track.visible ? '' : ''}" data-action="toggle-visibility" title="Toggle Visibility">
                        <svg viewBox="0 0 16 16" width="14" height="14"><circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" fill="none"/><circle cx="8" cy="8" r="3" fill="currentColor"/></svg>
                    </button>
                </div>
            `;
            this.trackHeaders.appendChild(header);

            // Bind track header events
            header.querySelectorAll('.track-btn').forEach(btn => {
                btn.addEventListener('click', () => this.handleTrackHeaderAction(btn.dataset.action, track.id));
            });
        });
    }

    /**
     * Handle track header button actions (lock/visibility).
     */
    handleTrackHeaderAction(action, trackId) {
        const track = this.trackManager.getTrack(trackId);
        if (!track) return;

        switch (action) {
            case 'lock-track':
                track.locked = !track.locked;
                break;
            case 'toggle-visibility':
                track.visible = !track.visible;
                break;
        }
        this.renderTrackHeaders();
    }

    // ============================================
    // Canvas Rendering
    // ============================================

    renderLoop() {
        this.render();
        requestAnimationFrame(() => this.renderLoop());
    }

    render() {
        if (!this.ctx || !this.canvas) return;
        const w = this.canvas.width;
        const h = this.canvas.height;

        // Clear
        this.ctx.clearRect(0, 0, w, h);

        // Background
        this.ctx.fillStyle = '#11111b';
        this.ctx.fillRect(0, 0, w, h);

        // Time ruler
        this.renderTimeRuler(w);

        // Track rows
        const tracks = this.trackManager.tracks;
        for (let i = 0; i < tracks.length; i++) {
            const y = 24 + i * 48;
            const track = tracks[i];

            // Track background
            this.ctx.fillStyle = i % 2 === 0 ? '#181825' : '#1e1e2e';
            this.ctx.fillRect(0, y, w, 48);

            // Track separator line
            this.ctx.strokeStyle = '#313244';
            this.ctx.lineWidth = 1;
            this.ctx.beginPath();
            this.ctx.moveTo(0, y + 48);
            this.ctx.lineTo(w, y + 48);
            this.ctx.stroke();

            // Render clips
            if (track.visible) {
                track.clips.forEach(clip => {
                    const clipX = clip.startTime * this.pixelsPerSecond;
                    const clipWidth = clip.duration * this.pixelsPerSecond;
                    const isSelected = clip.id === this.selectedClipId;
                    this.clipRenderer.renderClip(clip, clipX, y + 6, Math.max(clipWidth, 2), 36, isSelected);
                });
            }
        }

        // Playhead
        this.renderPlayhead(h);
    }

    renderTimeRuler(width) {
        this.ctx.fillStyle = '#181825';
        this.ctx.fillRect(0, 0, width, 24);

        this.ctx.strokeStyle = '#313244';
        this.ctx.lineWidth = 1;
        this.ctx.beginPath();
        this.ctx.moveTo(0, 23);
        this.ctx.lineTo(width, 23);
        this.ctx.stroke();

        // Time markers
        const secondsVisible = width / this.pixelsPerSecond;
        const interval = this.getTimeRulerInterval(secondsVisible);

        this.ctx.fillStyle = '#9399b2';
        this.ctx.font = '10px system-ui, sans-serif';
        this.ctx.textBaseline = 'top';

        for (let t = 0; t <= secondsVisible; t += interval) {
            const x = t * this.pixelsPerSecond;
            this.ctx.beginPath();
            this.ctx.moveTo(x, 16);
            this.ctx.lineTo(x, 23);
            this.ctx.stroke();
            this.ctx.fillText(this.formatTime(t), x + 2, 4);
        }
    }

    getTimeRulerInterval(secondsVisible) {
        if (secondsVisible < 10) return 1;
        if (secondsVisible < 30) return 5;
        if (secondsVisible < 120) return 10;
        if (secondsVisible < 600) return 30;
        return 60;
    }

    formatTime(seconds) {
        const mins = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${mins}:${secs.toString().padStart(2, '0')}`;
    }

    renderPlayhead(totalHeight) {
        const x = this.playheadTime * this.pixelsPerSecond;

        // Playhead line
        this.ctx.strokeStyle = '#f38ba8';
        this.ctx.lineWidth = 2;
        this.ctx.beginPath();
        this.ctx.moveTo(x, 0);
        this.ctx.lineTo(x, totalHeight);
        this.ctx.stroke();

        // Playhead triangle head
        this.ctx.fillStyle = '#f38ba8';
        this.ctx.beginPath();
        this.ctx.moveTo(x - 6, 0);
        this.ctx.lineTo(x + 6, 0);
        this.ctx.lineTo(x, 10);
        this.ctx.closePath();
        this.ctx.fill();
    }

    // ============================================
    // Mouse Interaction
    // ============================================

    bindEvents() {
        const canvas = this.canvas;
        if (!canvas) return;

        canvas.addEventListener('mousedown', (e) => this.onMouseDown(e));
        canvas.addEventListener('mousemove', (e) => this.onMouseMove(e));
        canvas.addEventListener('mouseup', (e) => this.onMouseUp(e));
        canvas.addEventListener('dblclick', (e) => this.onDoubleClick(e));

        // Zoom slider
        const zoomSlider = document.getElementById('timeline-zoom');
        if (zoomSlider) {
            zoomSlider.addEventListener('input', (e) => {
                this.pixelsPerSecond = 10 + (e.target.value / 100) * 140;
                window.FlowCutEventBus.publish(window.FlowCutEvents.TIMELINE_ZOOM_CHANGED, { level: e.target.value });
            });
        }

        // Time ruler click to move playhead
        const timeRuler = document.getElementById('time-ruler');
        if (timeRuler) {
            timeRuler.addEventListener('mousedown', (e) => {
                const rect = timeRuler.getBoundingClientRect();
                this.setPlayheadTime((e.clientX - rect.left) / this.pixelsPerSecond);
            });
        }
    }

    onMouseDown(e) {
        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        const time = x / this.pixelsPerSecond;

        // Check if clicking on time ruler area (top 24px)
        if (y < 24) {
            this.setPlayheadTime(time);
            return;
        }

        // Find which track and clip
        const trackIndex = Math.floor((y - 24) / 48);
        const track = this.trackManager.tracks[trackIndex];

        if (!track || track.locked) return;

        const clip = track.getClipAtTime(time);

        switch (this.interactionMode) {
            case 'select':
                if (clip) {
                    this.selectClip(clip.id, track.id);
                    // Check if near clip edges for trimming
                    const clipStartX = clip.startTime * this.pixelsPerSecond;
                    const clipEndX = clip.endTime * this.pixelsPerSecond;
                    if (x - clipStartX < 8) {
                        this.startDrag('trimLeft', clip, x);
                    } else if (clipEndX - x < 8) {
                        this.startDrag('trimRight', clip, x);
                    } else {
                        this.startDrag('move', clip, x);
                    }
                } else {
                    this.selectClip(null, null);
                    this.setPlayheadTime(time);
                }
                break;

            case 'razor':
                if (clip) {
                    this.splitClipAtTime(track.id, clip.id, time);
                }
                break;

            case 'ripple':
                if (clip) {
                    this.selectClip(clip.id, track.id);
                    this.startDrag('move', clip, x);
                }
                break;

            default:
                if (clip) {
                    this.selectClip(clip.id, track.id);
                    this.startDrag('move', clip, x);
                }
                break;
        }
    }

    onMouseMove(e) {
        if (!this.isDragging) return;
        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const time = x / this.pixelsPerSecond;

        const clip = this.trackManager.getTrack(this.dragTrackId)?.getClip(this.dragClipId);
        if (!clip) return;

        switch (this.dragType) {
            case 'move':
                const newStart = Math.max(0, time - this.dragStartTime);
                clip.move(this.snapTime(newStart));
                break;
            case 'trimLeft':
                const newStartTime = Math.max(0, this.snapTime(time));
                if (newStartTime < clip.endTime - 0.1) {
                    clip.trimLeft(newStartTime);
                }
                break;
            case 'trimRight':
                const newEndTime = this.snapTime(time);
                if (newEndTime > clip.startTime + 0.1) {
                    clip.trimRight(newEndTime);
                }
                break;
        }
    }

    onMouseUp(e) {
        if (this.isDragging) {
            const clip = this.trackManager.getTrack(this.dragTrackId)?.getClip(this.dragClipId);
            if (clip) {
                // Notify backend of clip change
                switch (this.dragType) {
                    case 'move':
                        ipc.moveClip(this.dragTrackId, this.dragClipId, clip.startTime);
                        break;
                    case 'trimLeft':
                    case 'trimRight':
                        ipc.trimClip(this.dragTrackId, this.dragClipId, clip.inPoint, clip.outPoint);
                        break;
                }
            }
            this.isDragging = false;
            this.dragType = null;
        }
    }

    onDoubleClick(e) {
        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        const time = x / this.pixelsPerSecond;
        const trackIndex = Math.floor((y - 24) / 48);
        const track = this.trackManager.tracks[trackIndex];

        if (track) {
            const clip = track.getClipAtTime(time);
            if (clip) {
                // Open clip properties
                window.FlowCutEventBus.publish(window.FlowCutEvents.TIMELINE_SELECTION_CHANGED, {
                    clipId: clip.id,
                    trackId: track.id,
                    clip: clip
                });
            }
        }
    }

    startDrag(type, clip, startX) {
        this.isDragging = true;
        this.dragType = type;
        this.dragClipId = clip.id;
        this.dragTrackId = clip.trackId;
        this.dragStartX = startX;
        this.dragStartTime = (startX / this.pixelsPerSecond) - clip.startTime;
    }

    // ============================================
    // Clip Operations
    // ============================================

    selectClip(clipId, trackId) {
        this.selectedClipId = clipId;
        this.selectedTrackId = trackId;

        // Update selection state on all clips
        this.trackManager.tracks.forEach(track => {
            track.clips.forEach(clip => {
                clip.selected = clip.id === clipId;
            });
        });

        window.FlowCutEventBus.publish(window.FlowCutEvents.TIMELINE_SELECTION_CHANGED, {
            clipId,
            trackId,
            clip: trackId ? this.trackManager.getTrack(trackId)?.getClip(clipId) : null
        });
    }

    splitClipAtTime(trackId, clipId, time) {
        ipc.splitClip(trackId, clipId, time).then(result => {
            if (result) {
                const track = this.trackManager.getTrack(trackId);
                const clip = track?.getClip(clipId);
                if (clip) {
                    const splitResult = clip.splitAt(time);
                    if (splitResult) {
                        track.removeClip(clipId);
                        track.addClip(splitResult[0]);
                        track.addClip(splitResult[1]);
                    }
                }
            }
        });
    }

    splitAtPlayhead() {
        if (!this.selectedClipId) {
            app.showToast('warning', 'No Selection', 'Select a clip before splitting.');
            return;
        }
        this.splitClipAtTime(this.selectedTrackId, this.selectedClipId, this.playheadTime);
    }

    deleteSelectedClip() {
        if (!this.selectedClipId) return;
        ipc.removeClipFromTrack(this.selectedTrackId, this.selectedClipId).then(() => {
            const track = this.trackManager.getTrack(this.selectedTrackId);
            if (track) track.removeClip(this.selectedClipId);
            this.selectClip(null, null);
        });
    }

    addClipFromMedia(mediaItem, trackId) {
        const track = this.trackManager.getTrack(trackId);
        if (!track) return;

        const clip = new Clip(
            'clip-' + Date.now(),
            mediaItem.id,
            trackId,
            this.playheadTime,
            mediaItem.duration || 5
        );
        clip.name = mediaItem.name;
        clip.mediaType = mediaItem.media_type?.toLowerCase() || 'video';

        track.addClip(clip);
        ipc.addClipToTrack(trackId, mediaItem.id, clip.startTime, clip.duration);
    }

    setPlayheadTime(time) {
        this.playheadTime = Math.max(0, time);
        window.FlowCutEventBus.publish(window.FlowCutEvents.TIMELINE_PLAYHEAD_MOVED, { time: this.playheadTime });
    }

    snapTime(time) {
        if (!this.snapEnabled) return time;
        // Snap to clip boundaries
        let snapped = time;
        let minDist = this.snapThreshold / this.pixelsPerSecond;

        this.trackManager.tracks.forEach(track => {
            track.clips.forEach(clip => {
                const dist = Math.abs(time - clip.startTime);
                if (dist < minDist) { minDist = dist; snapped = clip.startTime; }
                const distEnd = Math.abs(time - clip.endTime);
                if (distEnd < minDist) { minDist = distEnd; snapped = clip.endTime; }
            });
        });

        return snapped;
    }

    // ============================================
    // Zoom Controls
    // ============================================

    zoomIn() {
        this.pixelsPerSecond = Math.min(150, this.pixelsPerSecond * 1.2);
        const slider = document.getElementById('timeline-zoom');
        if (slider) slider.value = ((this.pixelsPerSecond - 10) / 140) * 100;
        this.resizeCanvas();
    }

    zoomOut() {
        this.pixelsPerSecond = Math.max(10, this.pixelsPerSecond / 1.2);
        const slider = document.getElementById('timeline-zoom');
        if (slider) slider.value = ((this.pixelsPerSecond - 10) / 140) * 100;
        this.resizeCanvas();
    }

    setInteractionMode(mode) {
        this.interactionMode = mode;
    }

    getTrackCount(type) {
        return this.trackManager.getTrackCount(type);
    }

    // ============================================
    // Project Load / Clear
    // ============================================

    loadFromProject(projectData) {
        // Load tracks and clips from project data
        this.trackManager.tracks = [];
        if (projectData.timeline && projectData.timeline.tracks) {
            projectData.timeline.tracks.forEach(trackData => {
                const track = this.trackManager.addTrack(trackData.track_type, trackData.name);
                track.id = trackData.id;
                if (trackData.clips) {
                    trackData.clips.forEach(clipData => {
                        const clip = new Clip(
                            clipData.id,
                            clipData.media_id,
                            trackData.id,
                            clipData.start_time,
                            clipData.duration
                        );
                        clip.name = clipData.name || '';
                        track.addClip(clip);
                    });
                }
            });
        }
        this.renderTrackHeaders();
        this.resizeCanvas();
    }

    clearTimeline() {
        this.trackManager.tracks = [];
        this.selectedClipId = null;
        this.selectedTrackId = null;
        this.playheadTime = 0;
        this.renderTrackHeaders();
        this.resizeCanvas();
    }

    initializeDefaultTracks() {
        this.initDefaultTracks();
    }
}

window.FlowCutTimeline = FlowCutTimeline;
