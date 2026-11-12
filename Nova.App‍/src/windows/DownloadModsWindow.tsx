import { BoxIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const downloadModsWindowMetadata: WindowMetadata = {
    id: "download-mods",
    title: "下载 Mod",
    Icon: BoxIcon,
    theme: {
        accent: "#fb923c",
        tile: "linear-gradient(180deg, #ea580c 0%, #7c2d12 100%)",
        ink: "#fff7ed",
    },
    initialPosition: { x: 24, y: 60 },
};

export function DownloadModsWindow() {
    return (
        <WindowFrame metadata={downloadModsWindowMetadata}>
            <WindowEmptyState icon={<BoxIcon />} label="暂无 Mod" />
        </WindowFrame>
    );
}
