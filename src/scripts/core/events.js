/**
 * FlowCut — Event Bus System
 * Provides a publish/subscribe pattern for cross-module communication.
 * All major UI and data events flow through this centralized bus.
 */

class EventBus {
    constructor() {
        this._listeners = {};
        this._onceListeners = {};
    }

    /**
     * Subscribe to an event with a callback function.
     * @param {string} event - Event type to listen for
     * @param {Function} callback - Function to call when event fires
     * @returns {Function} Unsubscribe function
     */
    subscribe(event, callback) {
        if (!this._listeners[event]) {
            this._listeners[event] = [];
        }
        this._listeners[event].push(callback);
        return () => {
            const idx = this._listeners[event].indexOf(callback);
            if (idx > -1) this._listeners[event].splice(idx, 1);
        };
    }

    /**
     * Subscribe to an event for one invocation only.
     * @param {string} event - Event type
     * @param {Function} callback - Function to call once
     */
    once(event, callback) {
        if (!this._onceListeners[event]) {
            this._onceListeners[event] = [];
        }
        this._onceListeners[event].push(callback);
    }

    /**
     * Publish an event with data to all subscribers.
     * @param {string} event - Event type to publish
     * @param {*} data - Data payload for the event
     */
    publish(event, data) {
        // Notify permanent listeners
        if (this._listeners[event]) {
            this._listeners[event].forEach(cb => {
                try { cb(data); }
                catch (err) { console.error(`EventBus error in ${event}:`, err); }
            });
        }
        // Notify one-time listeners and remove them
        if (this._onceListeners[event]) {
            this._onceListeners[event].forEach(cb => {
                try { cb(data); }
                catch (err) { console.error(`EventBus once error in ${event}:`, err); }
            });
            this._onceListeners[event] = [];
        }
    }

    /**
     * Remove all listeners for a specific event.
     * @param {string} event - Event type to clear
     */
    clear(event) {
        this._listeners[event] = [];
        this._onceListeners[event] = [];
    }

    /**
     * Remove all listeners for all events.
     */
    clearAll() {
        this._listeners = {};
        this._onceListeners = {};
    }
}

// ============================================
// Event Type Constants
// ============================================

const Events = {
    // Project lifecycle events
    PROJECT_CREATED: 'project:created',
    PROJECT_OPENED: 'project:opened',
    PROJECT_SAVED: 'project:saved',
    PROJECT_CLOSED: 'project:closed',
    PROJECT_INFO_UPDATED: 'project:infoUpdated',

    // Media events
    MEDIA_IMPORTED: 'media:imported',
    MEDIA_REMOVED: 'media:removed',
    MEDIA_SELECTED: 'media:selected',
    MEDIA_INFO_UPDATED: 'media:infoUpdated',
    MEDIA_SEARCH_CHANGED: 'media:searchChanged',
    MEDIA_FILTER_CHANGED: 'media:filterChanged',

    // Timeline events
    TIMELINE_STATE_UPDATED: 'timeline:stateUpdated',
    TIMELINE_TRACK_ADDED: 'timeline:trackAdded',
    TIMELINE_TRACK_REMOVED: 'timeline:trackRemoved',
    TIMELINE_CLIP_ADDED: 'timeline:clipAdded',
    TIMELINE_CLIP_REMOVED: 'timeline:clipRemoved',
    TIMELINE_CLIP_MOVED: 'timeline:clipMoved',
    TIMELINE_CLIP_SPLIT: 'timeline:clipSplit',
    TIMELINE_CLIP_TRIMMED: 'timeline:clipTrimmed',
    TIMELINE_TRANSITION_ADDED: 'timeline:transitionAdded',
    TIMELINE_TRANSITION_REMOVED: 'timeline:transitionRemoved',
    TIMELINE_SELECTION_CHANGED: 'timeline:selectionChanged',
    TIMELINE_PLAYHEAD_MOVED: 'timeline:playheadMoved',
    TIMELINE_ZOOM_CHANGED: 'timeline:zoomChanged',

    // Preview events
    PREVIEW_FRAME_RENDERED: 'preview:frameRendered',
    PREVIEW_PLAY_STARTED: 'preview:playStarted',
    PREVIEW_PLAY_STOPPED: 'preview:playStopped',
    PREVIEW_SEEK: 'preview:seek',
    PREVIEW_FRAME_CHANGED: 'preview:frameChanged',
    PREVIEW_QUALITY_CHANGED: 'preview:qualityChanged',

    // Effects / filter events
    EFFECT_APPLIED: 'effect:applied',
    EFFECT_REMOVED: 'effect:removed',
    EFFECT_PARAMS_UPDATED: 'effect:paramsUpdated',
    EFFECT_ENABLED_CHANGED: 'effect:enabledChanged',
    EFFECT_ORDER_CHANGED: 'effect:orderChanged',

    // Export events
    EXPORT_STARTED: 'export:started',
    EXPORT_PROGRESS_UPDATED: 'export:progressUpdated',
    EXPORT_COMPLETED: 'export:completed',
    EXPORT_FAILED: 'export:failed',
    EXPORT_CANCELLED: 'export:cancelled',

    // Engine events
    ENGINE_INITIALIZED: 'engine:initialized',
    ENGINE_STATUS_UPDATED: 'engine:statusUpdated',
    ENGINE_ERROR: 'engine:error',

    // UI events
    UI_PANEL_RESIZED: 'ui:panelResized',
    UI_PANEL_TOGGLED: 'ui:panelToggled',
    UI_MODAL_OPENED: 'ui:modalOpened',
    UI_MODAL_CLOSED: 'ui:modalClosed',
    UI_TOOL_CHANGED: 'ui:toolChanged',
    UI_CONTEXT_MENU: 'ui:contextMenu',
    UI_THEME_CHANGED: 'ui:themeChanged',

    // Undo/Redo events
    UNDO_PERFORMED: 'undo:performed',
    REDO_PERFORMED: 'redo:performed',
    UNDO_HISTORY_UPDATED: 'undo:historyUpdated',

    // Error events
    ERROR_OCCURRED: 'error:occurred',
    ERROR_IPC_FAILED: 'error:ipcFailed',
};

// Global event bus instance
const eventBus = new EventBus();

// Export for use in other modules
window.FlowCutEvents = Events;
window.FlowCutEventBus = eventBus;
