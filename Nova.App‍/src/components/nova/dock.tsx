import { useShallow } from "zustand/react/shallow";
import { default as Dock } from "@/components/bits/Dock";
import { hasMaximizedWindow, useWindowStore } from "@/stores/window-store";
import { WINDOW_METADATA } from "@/windows";

export function AppDock() {
    const shouldHide = useWindowStore((state) => hasMaximizedWindow(state.windows));
    const activeWindows = useWindowStore(
        useShallow((state) => WINDOW_METADATA.map(({ id }) => Boolean(state.windows[id]?.isOpen)))
    );
    const toggleDockWindow = useWindowStore((state) => state.toggleDockWindow);
    const items = WINDOW_METADATA.map((metadata, index) => {
        const Icon = metadata.Icon;
        return {
            icon: <Icon aria-hidden="true" />,
            label: metadata.title,
            theme: metadata.theme,
            active: activeWindows[index],
            onClick: () => toggleDockWindow(metadata.id, metadata.initialPosition),
        };
    });

    return (
        <Dock
            items={items}
            baseItemSize={50}
            magnification={60}
            panelHeight={64}
            accentColor="#0a84ff"
            badgeColor="#ff453a"
            position="bottom"
            distance={100}
            gap={10}
            roundness={0.5}
            bounce
            autoHide={shouldHide}
            tiles
            showLabels
            showIndicators
            showBadges
            className="z-50!"
        />
    );
}
