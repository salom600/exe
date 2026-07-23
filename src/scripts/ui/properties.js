/**
 * FlowCut — Properties Panel Controller
 * Manages clip properties editing, effects/filters, and color correction.
 */

class FlowCutProperties {
    constructor() {
        this.currentClip = null;
        this.currentMedia = null;
        this.effects = [];

        this.bindEvents();
    }

    /**
     * Bind properties panel UI events.
     */
    bindEvents() {
        // Collapse toggles
        document.querySelectorAll('.collapse-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const section = btn.dataset.section;
                const content = document.querySelector(`#${section} .section-content`);
                if (content) {
                    content.classList.toggle('collapsed');
                    btn.classList.toggle('collapsed');
                }
            });
        });

        // Clip property inputs
        const clipStart = document.getElementById('clip-start');
        const clipDuration = document.getElementById('clip-duration');
        const clipSpeed = document.getElementById('clip-speed');

        if (clipStart) clipStart.addEventListener('change', (e) => this.updateClipProperty('startTime', parseFloat(e.target.value)));
        if (clipDuration) clipDuration.addEventListener('change', (e) => this.updateClipProperty('duration', parseFloat(e.target.value)));
        if (clipSpeed) clipSpeed.addEventListener('change', (e) => this.updateClipProperty('speed', parseFloat(e.target.value)));

        // Color correction sliders
        ['brightness', 'contrast', 'saturation', 'hue'].forEach(param => {
            const slider = document.getElementById(param);
            if (slider) {
                slider.addEventListener('input', (e) => {
                    const valueSpan = e.target.parentElement.querySelector('.prop-value');
                    if (valueSpan) valueSpan.textContent = e.target.value;
                });
                slider.addEventListener('change', (e) => this.updateColorParam(param, parseInt(e.target.value)));
            }
        });

        // Add effect button
        const addEffectBtn = document.querySelector('[data-action="add-effect"]');
        if (addEffectBtn) {
            addEffectBtn.addEventListener('click', () => this.showAddEffectDialog());
        }

        // Clip selected from timeline
        window.FlowCutEventBus.subscribe(window.FlowCutEvents.TIMELINE_SELECTION_CHANGED, (data) => {
            this.onClipSelected(data);
        });

        // Media selected from browser
        window.FlowCutEventBus.subscribe(window.FlowCutEvents.MEDIA_SELECTED, (data) => {
            this.onMediaSelected(data);
        });
    }

    /**
     * Update a clip property via IPC.
     */
    async updateClipProperty(property, value) {
        if (!this.currentClip) return;
        // Update locally
        this.currentClip[property] = value;
        // Send to backend
        if (property === 'startTime') {
            await ipc.moveClip(this.currentClip.trackId, this.currentClip.id, value);
        } else if (property === 'duration') {
            await ipc.trimClip(this.currentClip.trackId, this.currentClip.id, this.currentClip.inPoint, value);
        }
    }

    /**
     * Update a color correction parameter.
     */
    updateColorParam(param, value) {
        if (!this.currentClip) return;
        // Apply brightness/contrast/saturation filter
        ipc.applyFilter(this.currentClip.id, param, { value }).then(result => {
            if (result) {
                window.FlowCutEventBus.publish(window.FlowCutEvents.EFFECT_PARAMS_UPDATED, { filterId: result.id, params: { value } });
            }
        });
    }

    /**
     * Handle clip selected from timeline.
     */
    onClipSelected(data) {
        if (!data.clip) {
            this.clearProperties();
            return;
        }

        this.currentClip = data.clip;
        this.populateClipProperties(data.clip);
    }

    /**
     * Handle media selected from browser.
     */
    onMediaSelected(data) {
        this.currentMedia = data.item;
        // Show media info in properties
        const clipName = document.getElementById('clip-name');
        if (clipName) {
            clipName.value = data.item.name || '';
            clipName.disabled = true;
        }
    }

    /**
     * Populate clip properties in the panel.
     */
    populateClipProperties(clip) {
        const clipName = document.getElementById('clip-name');
        const clipStart = document.getElementById('clip-start');
        const clipDuration = document.getElementById('clip-duration');
        const clipSpeed = document.getElementById('clip-speed');

        if (clipName) { clipName.value = clip.name || ''; clipName.disabled = false; }
        if (clipStart) { clipStart.value = clip.startTime?.toFixed(2) || '0'; clipStart.disabled = false; }
        if (clipDuration) { clipDuration.value = clip.duration?.toFixed(2) || '0'; clipDuration.disabled = false; }
        if (clipSpeed) { clipSpeed.value = clip.speed || '1.0'; clipSpeed.disabled = false; }

        // Enable color sliders
        ['brightness', 'contrast', 'saturation', 'hue'].forEach(id => {
            const slider = document.getElementById(id);
            if (slider) slider.disabled = false;
        });

        // Load clip effects
        this.loadEffects(clip.filters || []);
    }

    /**
     * Clear all properties when no clip is selected.
     */
    clearProperties() {
        this.currentClip = null;
        const clipName = document.getElementById('clip-name');
        const clipStart = document.getElementById('clip-start');
        const clipDuration = document.getElementById('clip-duration');
        const clipSpeed = document.getElementById('clip-speed');

        if (clipName) { clipName.value = ''; clipName.disabled = true; }
        if (clipStart) { clipStart.value = ''; clipStart.disabled = true; }
        if (clipDuration) { clipDuration.value = ''; clipDuration.disabled = true; }
        if (clipSpeed) { clipSpeed.value = '1.0'; clipSpeed.disabled = true; }

        ['brightness', 'contrast', 'saturation', 'hue'].forEach(id => {
            const slider = document.getElementById(id);
            if (slider) { slider.value = 0; slider.disabled = true; }
        });

        this.loadEffects([]);
    }

    /**
     * Load and display effects for the current clip.
     */
    loadEffects(filters) {
        this.effects = filters;
        const list = document.getElementById('effects-list');
        if (!list) return;

        if (filters.length === 0) {
            list.innerHTML = '<div class="effects-empty"><p>No effects applied</p></div>';
            return;
        }

        list.innerHTML = filters.map(filter => `
            <div class="effect-item" data-id="${filter.id}">
                <span class="effect-name">${filter.filter_type || 'Unknown Effect'}</span>
                <div class="effect-toggle ${filter.enabled ? '' : 'disabled'}" data-id="${filter.id}" title="${filter.enabled ? 'Enabled' : 'Disabled'}"></div>
                <button class="effect-delete" data-id="${filter.id}" title="Remove Effect">&#x2715;</button>
            </div>
        `).join('');

        // Bind effect toggle
        list.querySelectorAll('.effect-toggle').forEach(toggle => {
            toggle.addEventListener('click', () => this.toggleEffect(toggle.dataset.id));
        });

        // Bind effect delete
        list.querySelectorAll('.effect-delete').forEach(btn => {
            btn.addEventListener('click', () => this.removeEffect(btn.dataset.id));
        });

        // Bind effect selection
        list.querySelectorAll('.effect-item').forEach(item => {
            item.addEventListener('click', (e) => {
                if (!e.target.closest('.effect-toggle') && !e.target.closest('.effect-delete')) {
                    this.selectEffect(item.dataset.id);
                }
            });
        });
    }

    /**
     * Toggle effect enabled/disabled.
     */
    async toggleEffect(filterId) {
        await ipc.updateFilterParams(filterId, { enabled: !this.getEffect(filterId)?.enabled });
        this.loadEffects(this.effects);
    }

    /**
     * Remove an effect.
     */
    async removeEffect(filterId) {
        await ipc.removeFilter(filterId);
        this.effects = this.effects.filter(f => f.id !== filterId);
        this.loadEffects(this.effects);
        window.FlowCutEventBus.publish(window.FlowCutEvents.EFFECT_REMOVED, { filterId });
    }

    /**
     * Select an effect for parameter editing.
     */
    selectEffect(filterId) {
        document.querySelectorAll('.effect-item').forEach(item => {
            item.classList.toggle('selected', item.dataset.id === filterId);
        });

        const filter = this.getEffect(filterId);
        if (filter) {
            window.FlowCutEventBus.publish(window.FlowCutEvents.EFFECT_PARAMS_UPDATED, { filterId, params: filter.params });
        }
    }

    getEffect(filterId) {
        return this.effects.find(f => f.id === filterId);
    }

    /**
     * Show add effect dialog.
     */
    async showAddEffectDialog() {
        if (!this.currentClip) {
            app.showToast('warning', 'No Clip Selected', 'Select a clip before adding effects.');
            return;
        }

        const filters = await ipc.listFilters();
        // Show filter selection modal
        app.modules.dialogs?.showAddEffectModal(filters, this.currentClip.id);
    }
}

window.FlowCutProperties = FlowCutProperties;
