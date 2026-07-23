/**
 * FlowCut — Menu & Toolbar Controller
 * Manages menu bar, toolbar actions, and dropdown menu interactions.
 */

class FlowCutMenus {
    constructor() {
        this.activeMenu = null;
        this.bindEvents();
    }

    /**
     * Bind menu and toolbar events.
     */
    bindEvents() {
        // Menu bar items
        document.querySelectorAll('.menu-item').forEach(item => {
            item.addEventListener('mouseenter', () => {
                if (this.activeMenu) this.showMenu(item.dataset.menu);
            });
            item.addEventListener('click', () => {
                this.toggleMenu(item.dataset.menu);
            });
        });

        // Toolbar buttons
        document.querySelectorAll('#toolbar .tool-btn').forEach(btn => {
            if (btn.dataset.tool) {
                btn.addEventListener('click', () => app.setTool(btn.dataset.tool));
            } else if (btn.dataset.action) {
                btn.addEventListener('click', () => app.executeAction(btn.dataset.action));
            }
        });

        // Panel action buttons
        document.querySelectorAll('.panel-actions [data-action]').forEach(btn => {
            btn.addEventListener('click', () => app.executeAction(btn.dataset.action));
        });

        // Close menu on click outside
        document.addEventListener('click', (e) => {
            if (!e.target.closest('#menu-bar') && !e.target.closest('.dropdown')) {
                this.closeAllMenus();
            }
        });

        // Dropdown item actions
        document.querySelectorAll('.dropdown-item').forEach(item => {
            item.addEventListener('click', () => {
                app.executeAction(item.dataset.action);
                this.closeAllMenus();
            });
        });
    }

    /**
     * Toggle a dropdown menu visibility.
     */
    toggleMenu(menuName) {
        if (this.activeMenu === menuName) {
            this.closeAllMenus();
        } else {
            this.showMenu(menuName);
        }
    }

    /**
     * Show a specific dropdown menu and activate its menu item.
     */
    showMenu(menuName) {
        this.closeAllMenus();
        this.activeMenu = menuName;

        // Activate menu item
        document.querySelectorAll('.menu-item').forEach(item => {
            item.classList.toggle('active', item.dataset.menu === menuName);
        });

        // Show dropdown
        const dropdown = document.querySelector(`.dropdown[data-dropdown="${menuName}"]`);
        if (dropdown) {
            dropdown.classList.add('visible');
            // Position the dropdown under the menu item
            const menuItem = document.querySelector(`.menu-item[data-menu="${menuName}"]`);
            if (menuItem) {
                dropdown.style.left = menuItem.offsetLeft + 'px';
            }
        }
    }

    /**
     * Close all dropdown menus.
     */
    closeAllMenus() {
        this.activeMenu = null;
        document.querySelectorAll('.menu-item').forEach(item => item.classList.remove('active'));
        document.querySelectorAll('.dropdown').forEach(d => d.classList.remove('visible'));
    }
}

window.FlowCutMenus = FlowCutMenus;
