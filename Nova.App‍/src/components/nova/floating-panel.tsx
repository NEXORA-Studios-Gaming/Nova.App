import { useEffect, useLayoutEffect, useRef, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import { Maximize2, Minimize2, Minus, X } from "lucide-react";
import Draggable from "react-draggable";
import { cn } from "@/lib/utils";
import { useWindowStore } from "@/stores/window-store";
import type { WindowSize } from "@/stores/window-store";

type ResizeDirection = "n" | "e" | "s" | "w" | "ne" | "se" | "sw" | "nw";

const MAXIMIZED_POSITION = { x: 0, y: 0 };
const MIN_WINDOW_SIZE: WindowSize = { width: 240, height: 160 };

type Position = { x: number; y: number };

type FloatingPanelProps = {
    id: string;
    title?: string;
    name?: string;
    icon?: ReactNode;
    children: ReactNode;
    className?: string;
    initialPosition?: Position;
    initialSize?: WindowSize;
    initiallyOpen?: boolean;
    onClose?: () => void;
};

export function FloatingPanel({
    id,
    title,
    name,
    icon,
    children,
    className,
    initialPosition = { x: 24, y: 52 },
    initialSize = { width: 320, height: 220 },
    initiallyOpen = false,
    onClose,
}: FloatingPanelProps) {
    const nodeRef = useRef<HTMLElement>(null);
    const doubleClickCooldownRef = useRef(0);
    const restoreSizeRef = useRef<{ width: number; height: number } | null>(null);
    const windowState = useWindowStore((state) => state.windows[id]);
    const activeWindowId = useWindowStore((state) => state.activeWindowId);
    const registerWindow = useWindowStore((state) => state.registerWindow);
    const focusWindow = useWindowStore((state) => state.focusWindow);
    const restoreWindowForDrag = useWindowStore((state) => state.restoreWindowForDrag);
    const setWindowPosition = useWindowStore((state) => state.setWindowPosition);
    const setWindowBounds = useWindowStore((state) => state.setWindowBounds);
    const toggleWindowMaximized = useWindowStore((state) => state.toggleWindowMaximized);
    const toggleWindowMinimized = useWindowStore((state) => state.toggleWindowMinimized);
    const setWindowOpen = useWindowStore((state) => state.setWindowOpen);
    const windowName = name ?? title ?? "应用窗口";
    const position = windowState?.position ?? initialPosition;
    const size = windowState?.size ?? initialSize;
    const isMaximized = windowState?.isMaximized ?? false;
    const isVisible = windowState ? windowState.isOpen && !windowState.isMinimized : initiallyOpen;
    const initialX = initialPosition.x;
    const initialY = initialPosition.y;

    useEffect(() => {
        registerWindow(id, { x: initialX, y: initialY }, initiallyOpen, initialSize);
    }, [id, initialX, initialY, initiallyOpen, initialSize, registerWindow]);

    // function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    //     if (event.target !== event.currentTarget || isMaximized) return;

    //     let dx = 0;
    //     let dy = 0;

    //     switch (event.key) {
    //         case "ArrowLeft":
    //             dx = -1;
    //             break;
    //         case "ArrowRight":
    //             dx = 1;
    //             break;
    //         case "ArrowUp":
    //             dy = -1;
    //             break;
    //         case "ArrowDown":
    //             dy = 1;
    //             break;
    //         default:
    //             return;
    //     }

    //     const node = nodeRef.current;
    //     const parent = node?.parentElement;
    //     if (!node || !parent) return;

    //     event.preventDefault();
    //     const step = event.shiftKey ? 32 : 12;
    //     setWindowPosition(id, {
    //         x: Math.max(0, Math.min(position.x + dx * step, parent.clientWidth - node.offsetWidth)),
    //         y: Math.max(0, Math.min(position.y + dy * step, parent.clientHeight - node.offsetHeight)),
    //     });
    // }

    useLayoutEffect(() => {
        if (!isVisible || isMaximized) return;
        const node = nodeRef.current;
        if (!node) return;
        const rect = node.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
            restoreSizeRef.current = { width: rect.width, height: rect.height };
        }
    }, [isVisible, isMaximized]);

    function handleDragStart(event: MouseEvent) {
        if (!isMaximized) {
            focusWindow(id);
            return;
        }

        const node = nodeRef.current;
        const parent = node?.parentElement;
        if (!node || !parent) {
            toggleWindowMaximized(id);
            return;
        }

        const windowRect = node.getBoundingClientRect();
        const parentRect = parent.getBoundingClientRect();
        const size = restoreSizeRef.current;
        const restoredWidth = Math.min(size?.width ?? 320, parent.clientWidth);
        const restoredHeight = size?.height ?? node.offsetHeight;
        const pointerEvent = event as MouseEvent & { touches?: TouchList; changedTouches?: TouchList };
        const touch = pointerEvent.touches?.[0] ?? pointerEvent.changedTouches?.[0];
        const clientX = touch?.clientX ?? pointerEvent.clientX ?? 0;
        const clientY = touch?.clientY ?? pointerEvent.clientY ?? 0;
        const horizontalRatio = windowRect.width > 0 ? (clientX - windowRect.left) / windowRect.width : 0;
        const handleRect = node.querySelector<HTMLElement>("[data-floating-panel-handle]")?.getBoundingClientRect();
        const handleOffsetY = Math.max(0, Math.min(clientY - (handleRect?.top ?? windowRect.top), handleRect?.height ?? 40));
        const x = Math.max(
            0,
            Math.min(clientX - parentRect.left - horizontalRatio * restoredWidth, parent.clientWidth - restoredWidth)
        );
        const y = Math.max(0, Math.min(clientY - parentRect.top - handleOffsetY, parent.clientHeight - restoredHeight));

        restoreWindowForDrag(id, { x, y });
    }

    function handleResizeStart(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
        event.preventDefault();
        event.stopPropagation();
        focusWindow(id);

        const node = nodeRef.current;
        const parent = node?.parentElement;
        if (!node || !parent || isMaximized) return;

        const startX = event.clientX;
        const startY = event.clientY;
        const startPosition = position;
        const startSize = size;
        const onMove = (moveEvent: globalThis.PointerEvent) => {
            const dx = moveEvent.clientX - startX;
            const dy = moveEvent.clientY - startY;
            let x = startPosition.x;
            let y = startPosition.y;
            let width = startSize.width;
            let height = startSize.height;

            if (direction.includes("e")) width = startSize.width + dx;
            if (direction.includes("s")) height = startSize.height + dy;
            if (direction.includes("w")) {
                x = startPosition.x + dx;
                width = startSize.width - dx;
            }
            if (direction.includes("n")) {
                y = startPosition.y + dy;
                height = startSize.height - dy;
            }

            width = Math.max(MIN_WINDOW_SIZE.width, Math.min(width, parent.clientWidth));
            height = Math.max(MIN_WINDOW_SIZE.height, Math.min(height, parent.clientHeight));
            x = direction.includes("w")
                ? Math.max(0, Math.min(x, startPosition.x + startSize.width - MIN_WINDOW_SIZE.width))
                : startPosition.x;
            y = direction.includes("n")
                ? Math.max(0, Math.min(y, startPosition.y + startSize.height - MIN_WINDOW_SIZE.height))
                : startPosition.y;
            x = Math.min(x, parent.clientWidth - width);
            y = Math.min(y, parent.clientHeight - height);
            setWindowBounds(id, { position: { x, y }, size: { width, height } });
        };
        const onUp = () => {
            window.removeEventListener("pointermove", onMove);
            window.removeEventListener("pointerup", onUp);
            window.removeEventListener("pointercancel", onUp);
        };

        window.addEventListener("pointermove", onMove);
        window.addEventListener("pointerup", onUp);
        window.addEventListener("pointercancel", onUp);
    }

    function closePanel() {
        setWindowOpen(id, false);
        onClose?.();
    }

    if (!isVisible) return null;

    return (
        <Draggable
            bounds="parent"
            cancel="button, a, input, textarea, select, [data-no-drag]"
            handle="[data-floating-panel-handle]"
            nodeRef={nodeRef}
            position={isMaximized ? MAXIMIZED_POSITION : position}
            onStart={handleDragStart}
            onDrag={(_, data) => setWindowPosition(id, { x: data.x, y: data.y })}>
            <section
                ref={nodeRef}
                onPointerDown={(event) => {
                    if (event.target instanceof Element && event.target.closest("[data-resize-handle]")) return;
                    focusWindow(id);
                }}
                style={{
                    ...(isMaximized ? {} : { width: size.width, height: size.height }),
                    zIndex: activeWindowId === id ? 40 : 30,
                }}
                className={cn(
                    "absolute top-0 left-0 z-30 flex max-h-full w-80 max-w-[calc(100%-1rem)] flex-col overflow-hidden rounded-lg border border-border/70 bg-background/95 text-foreground shadow-xl backdrop-blur-xl",
                    isMaximized && "h-full! w-full! max-w-none!",
                    className
                )}>
                <div
                    data-floating-panel-handle
                    role="group"
                    tabIndex={0}
                    aria-label={`${windowName} 窗口标题栏`}
                    aria-keyshortcuts="ArrowLeft ArrowRight ArrowUp ArrowDown"
                    onDoubleClick={(event) => {
                        if (event.target instanceof Element && event.target.closest("button")) return;
                        const now = performance.now();
                        if (now - doubleClickCooldownRef.current < 400) return;
                        doubleClickCooldownRef.current = now;
                        toggleWindowMaximized(id);
                    }}
                    className="flex h-10 shrink-0 touch-none items-center gap-2 border-b border-border/70 bg-muted/60 pl-3 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring">
                    {icon ? (
                        <span className="flex size-4 shrink-0 items-center justify-center [&_svg]:size-4">{icon}</span>
                    ) : null}
                    <span className="min-w-0 flex-1 truncate text-sm font-semibold">{windowName}</span>
                    <div className="flex h-full shrink-0 items-center border-l border-border/70">
                        <button
                            type="button"
                            aria-label="最小化窗口"
                            title="最小化"
                            onClick={() => toggleWindowMinimized(id)}
                            className="flex h-full w-10 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-ring">
                            <Minus aria-hidden="true" className="size-3.5" />
                        </button>
                        <button
                            type="button"
                            aria-label={isMaximized ? "还原窗口大小" : "最大化窗口"}
                            title={isMaximized ? "还原" : "最大化"}
                            aria-pressed={isMaximized}
                            onClick={() => toggleWindowMaximized(id)}
                            className="flex h-full w-10 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-ring">
                            {isMaximized ? (
                                <Minimize2 aria-hidden="true" className="size-3.5" />
                            ) : (
                                <Maximize2 aria-hidden="true" className="size-3.5" />
                            )}
                        </button>
                        <button
                            type="button"
                            aria-label="关闭窗口"
                            title="关闭"
                            onClick={closePanel}
                            className="flex h-full w-10 items-center justify-center text-muted-foreground transition-colors hover:bg-destructive hover:text-white focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-ring">
                            <X aria-hidden="true" className="size-4" />
                        </button>
                    </div>
                </div>
                {(["n", "e", "s", "w", "ne", "se", "sw", "nw"] as ResizeDirection[]).map((direction) => (
                    <div
                        key={direction}
                        data-no-drag
                        aria-hidden="true"
                        data-resize-handle
                        onPointerDown={(event) => handleResizeStart(event, direction)}
                        className={cn(
                            "absolute z-10 touch-none select-none",
                            direction === "n" && "inset-x-2 top-0 h-1 cursor-n-resize",
                            direction === "e" && "inset-y-2 right-0 w-1 cursor-e-resize",
                            direction === "s" && "inset-x-2 bottom-0 h-1 cursor-s-resize",
                            direction === "w" && "inset-y-2 left-0 w-1 cursor-w-resize",
                            direction === "ne" && "top-0 right-0 size-3 cursor-ne-resize",
                            direction === "se" && "right-0 bottom-0 size-3 cursor-se-resize",
                            direction === "sw" && "bottom-0 left-0 size-3 cursor-sw-resize",
                            direction === "nw" && "top-0 left-0 size-3 cursor-nw-resize"
                        )}
                    />
                ))}
                <div className="min-h-0 flex-1 overflow-auto p-4">{children}</div>
            </section>
        </Draggable>
    );
}
