import { GlobeIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const downloadWorldsWindowMetadata: WindowMetadata = {
    id: "download-worlds",
    title: "下载世界",
    Icon: GlobeIcon,
    theme: {
        accent: "#7dd3fc",
        tile: "linear-gradient(180deg, #0284c7 0%, #0c4a6e 100%)",
        ink: "#e0f2fe",
    },
    initialPosition: { x: 24, y: 100 },
};

export function DownloadWorldsWindow() {
    return (
        <WindowFrame metadata={downloadWorldsWindowMetadata}>
            <WindowEmptyState icon={<GlobeIcon />} label="暂无世界" />
        </WindowFrame>
    );
}
