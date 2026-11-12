import { create } from "zustand";

export type WindowPosition = { x: number; y: number };
export type WindowSize = { width: number; height: number };
export type WindowBounds = { position: WindowPosition; size: WindowSize };

export type WindowState = WindowBounds & {
    isMaximized: boolean;
    isMinimized: boolean;
    isOpen: boolean;
};

type WindowStore = {
    windows: Record<string, WindowState>;
    activeWindowId: string | null;
    registerWindow: (id: string, initialPosition: WindowPosition, initiallyOpen?: boolean, initialSize?: WindowSize) => void;
    focusWindow: (id: string) => void;
    restoreWindowForDrag: (id: string, position: WindowPosition) => void;
    setWindowPosition: (id: string, position: WindowPosition) => void;
    setWindowBounds: (id: string, bounds: WindowBounds) => void;
    toggleWindowMaximized: (id: string) => void;
    toggleWindowMinimized: (id: string) => void;
    toggleDockWindow: (id: string, initialPosition: WindowPosition) => void;
    setWindowOpen: (id: string, isOpen: boolean, initialPosition?: WindowPosition) => void;
};

const DEFAULT_POSITION = { x: 24, y: 52 };
const DEFAULT_SIZE = { width: 320, height: 220 };

function createWindowState(position: WindowPosition, isOpen: boolean, size: WindowSize = DEFAULT_SIZE): WindowState {
    return { position, size, isMaximized: false, isMinimized: false, isOpen };
}

export const useWindowStore = create<WindowStore>((set) => ({
    windows: {},
    activeWindowId: null,
    registerWindow: (id, initialPosition, initiallyOpen = false, initialSize = DEFAULT_SIZE) =>
        set((state) => {
            if (state.windows[id]) return state;
            return {
                windows: { ...state.windows, [id]: createWindowState(initialPosition, initiallyOpen, initialSize) },
                activeWindowId: initiallyOpen ? (state.activeWindowId ?? id) : state.activeWindowId,
            };
        }),
    focusWindow: (id) => set((state) => (state.windows[id]?.isOpen ? { activeWindowId: id } : state)),
    restoreWindowForDrag: (id, position) =>
        set((state) => {
            const windowState = state.windows[id];
            if (!windowState?.isOpen) return state;
            return {
                windows: {
                    ...state.windows,
                    [id]: { ...windowState, position, isMaximized: false, isMinimized: false },
                },
                activeWindowId: id,
            };
        }),
    setWindowPosition: (id, position) =>
        set((state) => {
            const windowState = state.windows[id];
            if (!windowState) return state;
            return { windows: { ...state.windows, [id]: { ...windowState, position } } };
        }),
    setWindowBounds: (id, bounds) =>
        set((state) => {
            const windowState = state.windows[id];
            if (!windowState) return state;
            return { windows: { ...state.windows, [id]: { ...windowState, ...bounds } } };
        }),
    toggleWindowMaximized: (id) =>
        set((state) => {
            const windowState = state.windows[id];
            if (!windowState?.isOpen) return state;
            return {
                windows: {
                    ...state.windows,
                    [id]: { ...windowState, isMaximized: !windowState.isMaximized, isMinimized: false },
                },
                activeWindowId: id,
            };
        }),
    toggleWindowMinimized: (id) =>
        set((state) => {
            const windowState = state.windows[id];
            if (!windowState?.isOpen) return state;
            return {
                windows: {
                    ...state.windows,
                    [id]: { ...windowState, isMaximized: false, isMinimized: !windowState.isMinimized },
                },
                activeWindowId: windowState.isMinimized ? id : state.activeWindowId === id ? null : state.activeWindowId,
            };
        }),
    toggleDockWindow: (id, initialPosition) =>
        set((state) => {
            const windowState = state.windows[id];
            if (!windowState) {
                return {
                    windows: { ...state.windows, [id]: createWindowState(initialPosition, true) },
                    activeWindowId: id,
                };
            }
            if (!windowState.isOpen) {
                return {
                    windows: {
                        ...state.windows,
                        [id]: { ...windowState, isOpen: true, isMaximized: false, isMinimized: false },
                    },
                    activeWindowId: id,
                };
            }
            if (windowState.isMinimized) {
                return { windows: { ...state.windows, [id]: { ...windowState, isMinimized: false } }, activeWindowId: id };
            }
            return {
                windows: { ...state.windows, [id]: { ...windowState, isMaximized: false, isMinimized: true } },
                activeWindowId: state.activeWindowId === id ? null : state.activeWindowId,
            };
        }),
    setWindowOpen: (id, isOpen, initialPosition = DEFAULT_POSITION) =>
        set((state) => {
            const windowState = state.windows[id];
            if (!windowState) {
                if (!isOpen) return state;
                return { windows: { ...state.windows, [id]: createWindowState(initialPosition, true) }, activeWindowId: id };
            }
            return {
                windows: {
                    ...state.windows,
                    [id]: { ...windowState, isOpen, isMaximized: isOpen && windowState.isMaximized, isMinimized: false },
                },
                activeWindowId: isOpen ? id : state.activeWindowId === id ? null : state.activeWindowId,
            };
        }),
}));

export function hasMaximizedWindow(windows: Record<string, WindowState>) {
    return Object.values(windows).some((windowState) => windowState.isOpen && windowState.isMaximized);
}
