/**
 * FlowCut — Track Model & Management
 * Represents a single track (video/audio/text) in the timeline.
 */

class Track {
    constructor(id, type, name) {
        this.id = id;
        this.type = type; // 'Video', 'Audio', 'Text'
        this.name = name;
        this.clips = [];
        this.locked = false;
        this.visible = true;
        this.volume = 1.0;
    }

    addClip(clip) {
        this.clips.push(clip);
        this.sortClips();
    }

    removeClip(clipId) {
        this.clips = this.clips.filter(c => c.id !== clipId);
    }

    getClip(clipId) {
        return this.clips.find(c => c.id === clipId);
    }

    sortClips() {
        this.clips.sort((a, b) => a.startTime - b.startTime);
    }

    getTotalDuration() {
        if (this.clips.length === 0) return 0;
        const lastClip = this.clips[this.clips.length - 1];
        return lastClip.startTime + lastClip.duration;
    }

    getClipAtTime(time) {
        return this.clips.find(c => time >= c.startTime && time < c.startTime + c.duration);
    }
}

class TrackManager {
    constructor() {
        this.tracks = [];
    }

    addTrack(type, name) {
        const id = type === 'Video' ? `V${this.getTrackCount(type) + 1}`
                   : type === 'Audio' ? `A${this.getTrackCount(type) + 1}`
                   : `T${this.getTrackCount(type) + 1}`;
        const track = new Track(id, type, name);
        this.tracks.push(track);
        return track;
    }

    removeTrack(trackId) {
        this.tracks = this.tracks.filter(t => t.id !== trackId);
    }

    getTrack(trackId) {
        return this.tracks.find(t => t.id === trackId);
    }

    getTrackCount(type) {
        if (!type) return this.tracks.length;
        return this.tracks.filter(t => t.type === type).length;
    }

    getVideoTracks() {
        return this.tracks.filter(t => t.type === 'Video');
    }

    getAudioTracks() {
        return this.tracks.filter(t => t.type === 'Audio');
    }

    getTrackAtPosition(y) {
        const trackHeight = 48;
        const index = Math.floor(y / trackHeight);
        return this.tracks[index] || null;
    }
}

window.Track = Track;
window.TrackManager = TrackManager;
