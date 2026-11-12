import { SettingsIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const settingsWindowMetadata: WindowMetadata = {
    id: "settings",
    title: "设置",
    Icon: SettingsIcon,
    theme: {
        accent: "#a1a1aa",
        tile: "linear-gradient(180deg, #52525b 0%, #27272a 100%)",
        ink: "#f4f4f5",
    },
    initialPosition: { x: 24, y: 116 },
};

export function SettingsWindow() {
    return (
        <WindowFrame metadata={settingsWindowMetadata}>
            <WindowEmptyState icon={<SettingsIcon />} label="默认设置" />
        </WindowFrame>
    );
}
