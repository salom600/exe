/**
 * FlowCut — Preview Controller
 * Manages video preview playback, frame rendering, and transport controls.
 */

class FlowCutPreview {
    constructor() {
        this.canvas = document.getElementById('preview-canvas');
        this.ctx = this.canvas?.getContext('2d');
        this.currentFrame = null;
        this.isPlaying = false;
        this.currentTime = 0;
        this.totalDuration = 0;
        this.frameRate = 30;
        this.volume = 80;
        this.quality = 'half';
        this.isFullscreen = false;

        this.bindEvents();
    }

    /**
     * Bind preview control events.
     */
    bindEvents() {
        // Transport controls
        document.querySelectorAll('[data-action="play"], [data-action="stop"], [data-action="prev-frame"], [data-action="next-frame"]').forEach(btn => {
            btn.addEventListener('click', () => this.handleTransportAction(btn.dataset.action));
        });

        // Volume slider
        const volumeSlider = document.getElementById('volume-slider');
        if (volumeSlider) {
            volumeSlider.addEventListener('input', (e) => {
                this.volume = parseInt(e.target.value);
            });
        }

        // Preview quality select
        const qualitySelect = document.getElementById('preview-quality');
        if (qualitySelect) {
            qualitySelect.addEventListener('change', (e) => {
                this.quality = e.target.value;
                window.FlowCutEventBus.publish(window.FlowCutEvents.PREVIEW_QUALITY_CHANGED, { quality: this.quality });
            });
        }

        // Playhead move from timeline
        window.FlowCutEventBus.subscribe(window.FlowCutEvents.TIMELINE_PLAYHEAD_MOVED, (data) => {
            this.onPlayheadMoved(data);
        });
    }

    /**
     * Handle transport control actions.
     */
    handleTransportAction(action) {
        switch (action) {
            case 'play':
                this.togglePlayPause();
                break;
            case 'stop':
                this.stop();
                break;
            case 'prev-frame':
                this.stepBackward();
                break;
            case 'next-frame':
                this.stepForward();
                break;
        }
    }

    /**
     * Toggle between play and pause.
     */
    togglePlayPause() {
        if (this.isPlaying) {
            this.pause();
        } else {
            this.play();
        }
    }

    /**
     * Start playback from current position.
     */
    play() {
        this.isPlaying = true;
        const playBtn = document.getElementById('play-btn');
        const playIcon = document.getElementById('play-icon');

        if (playBtn) playBtn.classList.add('playing');
        if (playIcon) {
            playIcon.innerHTML = '<rect x="7" y="5" width="3" height="14" fill="currentColor"/><rect x="14" y="5" width="3" height="14" fill="currentColor"/>';
        }

        window.FlowCutEventBus.publish(window.FlowCutEvents.PREVIEW_PLAY_STARTED, { time: this.currentTime });
        this.startPlaybackLoop();
    }

    /**
     * Pause playback.
     */
    pause() {
        this.isPlaying = false;
        const playBtn = document.getElementById('play-btn');
        const playIcon = document.getElementById('play-icon');

        if (playBtn) playBtn.classList.remove('playing');
        if (playIcon) {
            playIcon.innerHTML = '<path d="M8 5L20 12L8 19Z" fill="currentColor"/>';
        }

        window.FlowCutEventBus.publish(window.FlowCutEvents.PREVIEW_PLAY_STOPPED, { time: this.currentTime });
    }

    /**
     * Stop playback and reset to beginning.
     */
    stop() {
        this.pause();
        this.currentTime = 0;
        this.updateTimecodeDisplay();
        this.renderFrameAt(0);
    }

    /**
     * Advance one frame forward.
     */
    stepForward() {
        this.currentTime += 1 / this.frameRate;
        if (this.currentTime > this.totalDuration) this.currentTime = this.totalDuration;
        this.renderFrameAt(this.currentTime);
        this.updateTimecodeDisplay();
    }

    /**
     * Go back one frame.
     */
    stepBackward() {
        this.currentTime -= 1 / this.frameRate;
        if (this.currentTime < 0) this.currentTime = 0;
        this.renderFrameAt(this.currentTime);
        this.updateTimecodeDisplay();
    }

    /**
     * Playback loop that renders frames continuously.
     */
    startPlaybackLoop() {
        const interval = 1000 / this.frameRate;
        this._playbackTimer = setInterval(() => {
            if (!this.isPlaying) {
                clearInterval(this._playbackTimer);
                return;
            }
            this.currentTime += 1 / this.frameRate;
            if (this.currentTime >= this.totalDuration) {
                this.pause();
                return;
            }
            this.renderFrameAt(this.currentTime);
            this.updateTimecodeDisplay();
        }, interval);
    }

    /**
     * Render a preview frame at the given timestamp.
     */
    async renderFrameAt(timestamp) {
        try {
            const frameData = await ipc.renderPreviewFrame(timestamp);
            if (frameData && this.ctx) {
                // Draw base64 image data on canvas
                const img = new Image();
                img.onload = () => {
                    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
                    this.ctx.drawImage(img, 0, 0, this.canvas.width, this.canvas.height);
                    // Hide overlay
                    const overlay = document.getElementById('no-media-overlay');
                    if (overlay) overlay.style.display = 'none';
                };
                img.src = `data:image/png;base64,${frameData}`;
            }
        } catch (e) {
            // If IPC call fails, render a placeholder frame
            this.renderPlaceholderFrame(timestamp);
        }
    }

    /**
     * Render a placeholder frame when no video data is available.
     */
    renderPlaceholderFrame(timestamp) {
        if (!this.ctx) return;
        const w = this.canvas.width;
        const h = this.canvas.height;

        this.ctx.fillStyle = '#000';
        this.ctx.fillRect(0, 0, w, h);

        // Draw time display
        this.ctx.fillStyle = '#89b4fa';
        this.ctx.font = '24px system-ui, sans-serif';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText(this.formatTimecode(timestamp), w / 2, h / 2);
    }

    /**
     * Handle playhead moved from timeline.
     */
    onPlayheadMoved(data) {
        this.currentTime = data.time;
        this.renderFrameAt(data.time);
        this.updateTimecodeDisplay();
    }

    /**
     * Update the timecode display in the UI.
     */
    updateTimecodeDisplay() {
        const currentTC = document.getElementById('current-timecode');
        const totalTC = document.getElementById('total-timecode');
        if (currentTC) currentTC.textContent = this.formatTimecode(this.currentTime);
        if (totalTC) totalTC.textContent = this.formatTimecode(this.totalDuration);
    }

    /**
     * Format a time value as timecode (HH:MM:SS:FF).
     */
    formatTimecode(seconds) {
        if (!seconds || seconds < 0) seconds = 0;
        const hrs = Math.floor(seconds / 3600);
        const mins = Math.floor((seconds % 3600) / 60);
        const secs = Math.floor(seconds % 60);
        const frames = Math.floor((seconds % 1) * this.frameRate);
        return `${hrs.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}:${frames.toString().padStart(2, '0')}`;
    }

    /**
     * Toggle fullscreen mode for the preview monitor.
     */
    toggleFullscreen() {
        const previewArea = document.getElementById('preview-area');
        if (!previewArea) return;

        this.isFullscreen = !this.isFullscreen;
        if (this.isFullscreen) {
            previewArea.classList.add('fullscreen');
            if (previewArea.requestFullscreen) previewArea.requestFullscreen();
        } else {
            previewArea.classList.remove('fullscreen');
            if (document.exitFullscreen) document.exitFullscreen();
        }
    }

    /**
     * Clear the preview when no project is loaded.
     */
    clearPreview() {
        if (this.ctx) {
            this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        }
        this.currentTime = 0;
        this.totalDuration = 0;
        this.isPlaying = false;
        this.updateTimecodeDisplay();
        const overlay = document.getElementById('no-media-overlay');
        if (overlay) overlay.style.display = '';
    }

    onFrameRendered(data) {
        // Update preview canvas with rendered frame
        this.renderFrameAt(this.currentTime);
    }
}

window.FlowCutPreview = FlowCutPreview;
