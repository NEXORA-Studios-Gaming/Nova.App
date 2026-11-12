import { LensIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const downloadResourcePacksWindowMetadata: WindowMetadata = {
    id: "download-resource-packs",
    title: "下载资源包",
    Icon: LensIcon,
    theme: {
        accent: "#67e8f9",
        tile: "linear-gradient(180deg, #0891b2 0%, #164e63 100%)",
        ink: "#cffafe",
    },
    initialPosition: { x: 24, y: 76 },
};

export function DownloadResourcePacksWindow() {
    return (
        <WindowFrame metadata={downloadResourcePacksWindowMetadata}>
            <WindowEmptyState icon={<LensIcon />} label="暂无资源包" />
        </WindowFrame>
    );
}
