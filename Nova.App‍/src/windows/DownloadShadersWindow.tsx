import { GalaxyIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const downloadShadersWindowMetadata: WindowMetadata = {
    id: "download-shaders",
    title: "下载光影包",
    Icon: GalaxyIcon,
    theme: {
        accent: "#f9a8d4",
        tile: "linear-gradient(180deg, #db2777 0%, #500724 100%)",
        ink: "#fce7f3",
    },
    initialPosition: { x: 24, y: 84 },
};

export function DownloadShadersWindow() {
    return (
        <WindowFrame metadata={downloadShadersWindowMetadata}>
            <WindowEmptyState icon={<GalaxyIcon />} label="暂无光影包" />
        </WindowFrame>
    );
}
