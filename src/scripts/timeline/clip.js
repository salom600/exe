/**
 * FlowCut — Clip Model & Rendering
 * Represents a single clip on a timeline track.
 */

class Clip {
    constructor(id, mediaId, trackId, startTime, duration) {
        this.id = id;
        this.mediaId = mediaId;
        this.trackId = trackId;
        this.startTime = startTime;
        this.duration = duration;
        this.inPoint = 0;
        this.outPoint = duration;
        this.speed = 1.0;
        this.filters = [];
        this.transitions = [];
        this.name = '';
        this.mediaType = 'video'; // 'video', 'audio', 'image'
        this.selected = false;
    }

    get endTime() {
        return this.startTime + this.duration;
    }

    trimLeft(newStartTime) {
        const trimAmount = newStartTime - this.startTime;
        this.startTime = newStartTime;
        this.inPoint += trimAmount;
        this.duration -= trimAmount;
    }

    trimRight(newEndTime) {
        const trimAmount = newEndTime - this.endTime;
        this.outPoint -= trimAmount;
        this.duration += trimAmount;
    }

    move(newStartTime) {
        this.startTime = newStartTime;
    }

    splitAt(splitTime) {
        if (splitTime <= this.startTime || splitTime >= this.endTime) return null;

        const leftDuration = splitTime - this.startTime;
        const rightDuration = this.endTime - splitTime;

        const leftClip = new Clip(
            this.id + '-L',
            this.mediaId,
            this.trackId,
            this.startTime,
            leftDuration
        );
        leftClip.inPoint = this.inPoint;
        leftClip.outPoint = this.inPoint + leftDuration;
        leftClip.speed = this.speed;
        leftClip.name = this.name;
        leftClip.mediaType = this.mediaType;

        const rightClip = new Clip(
            this.id + '-R',
            this.mediaId,
            this.trackId,
            splitTime,
            rightDuration
        );
        rightClip.inPoint = this.inPoint + leftDuration;
        rightClip.outPoint = this.outPoint;
        rightClip.speed = this.speed;
        rightClip.name = this.name;
        rightClip.mediaType = this.mediaType;

        return [leftClip, rightClip];
    }

    clone() {
        const cloned = new Clip(this.id, this.mediaId, this.trackId, this.startTime, this.duration);
        cloned.inPoint = this.inPoint;
        cloned.outPoint = this.outPoint;
        cloned.speed = this.speed;
        cloned.name = this.name;
        cloned.mediaType = this.mediaType;
        cloned.filters = [...this.filters];
        cloned.transitions = [...this.transitions];
        return cloned;
    }
}

class ClipRenderer {
    constructor(ctx) {
        this.ctx = ctx;
        this.colors = {
            video: { bg: 'rgba(137, 180, 250, 0.6)', border: '#89b4fa', text: '#cdd6f4' },
            audio: { bg: 'rgba(166, 227, 161, 0.4)', border: '#a6e3a1', text: '#cdd6f4' },
            image: { bg: 'rgba(203, 166, 247, 0.5)', border: '#cba6f7', text: '#cdd6f4' },
        };
    }

    /**
     * Render a clip on the timeline canvas.
     */
    renderClip(clip, x, y, width, height, isSelected) {
        const colors = this.colors[clip.mediaType] || this.colors.video;

        // Clip background
        this.ctx.fillStyle = colors.bg;
        this.ctx.beginPath();
        this.ctx.roundRect(x, y, width, height, 4);
        this.ctx.fill();

        // Clip border
        this.ctx.strokeStyle = isSelected ? '#89b4fa' : colors.border;
        this.ctx.lineWidth = isSelected ? 2 : 1;
        this.ctx.beginPath();
        this.ctx.roundRect(x, y, width, height, 4);
        this.ctx.stroke();

        // Selection glow
        if (isSelected) {
            this.ctx.shadowColor = '#89b4fa';
            this.ctx.shadowBlur = 8;
            this.ctx.strokeStyle = '#89b4fa';
            this.ctx.lineWidth = 2;
            this.ctx.beginPath();
            this.ctx.roundRect(x, y, width, height, 4);
            this.ctx.stroke();
            this.ctx.shadowBlur = 0;
        }

        // Clip name
        if (width > 40) {
            this.ctx.fillStyle = colors.text;
            this.ctx.font = '11px system-ui, sans-serif';
            this.ctx.textBaseline = 'middle';
            const nameText = clip.name || 'Clip';
            const maxWidth = width - 16;
            const truncated = this.truncateText(nameText, maxWidth);
            this.ctx.fillText(truncated, x + 8, y + height / 2);
        }

        // Trim handles (small rectangles at edges)
        if (width > 20 && isSelected) {
            this.ctx.fillStyle = 'rgba(255, 255, 255, 0.3)';
            this.ctx.fillRect(x, y, 6, height);
            this.ctx.fillRect(x + width - 6, y, 6, height);
        }

        // Audio waveform placeholder for audio clips
        if (clip.mediaType === 'audio' && width > 60) {
            this.drawWaveformPlaceholder(x + 8, y + 4, width - 16, height - 8);
        }

        // Thumbnail strip for video clips
        if (clip.mediaType === 'video' && width > 100) {
            this.drawThumbnailStrip(x + 8, y + 4, width - 16, height - 8);
        }
    }

    truncateText(text, maxWidth) {
        const measured = this.ctx.measureText(text);
        if (measured.width <= maxWidth) return text;
        let truncated = text;
        while (this.ctx.measureText(truncated + '...').width > maxWidth && truncated.length > 0) {
            truncated = truncated.slice(0, -1);
        }
        return truncated + '...';
    }

    drawWaveformPlaceholder(x, y, width, height) {
        this.ctx.strokeStyle = 'rgba(166, 227, 161, 0.5)';
        this.ctx.lineWidth = 1;
        this.ctx.beginPath();
        const centerY = y + height / 2;
        for (let i = 0; i < width; i += 4) {
            const amplitude = Math.random() * height * 0.4;
            this.ctx.moveTo(x + i, centerY - amplitude);
            this.ctx.lineTo(x + i, centerY + amplitude);
        }
        this.ctx.stroke();
    }

    drawThumbnailStrip(x, y, width, height) {
        // Draw alternating light/dark rectangles to simulate thumbnail strip
        const thumbWidth = 24;
        const count = Math.floor(width / thumbWidth);
        for (let i = 0; i < count; i++) {
            const brightness = 0.15 + (i % 2) * 0.1;
            this.ctx.fillStyle = `rgba(137, 180, 250, ${brightness})`;
            this.ctx.fillRect(x + i * thumbWidth, y, thumbWidth - 1, height);
        }
    }
}

window.Clip = Clip;
window.ClipRenderer = ClipRenderer;
