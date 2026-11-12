import { PackageIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const downloadModpacksWindowMetadata: WindowMetadata = {
    id: "download-modpacks",
    title: "下载整合包",
    Icon: PackageIcon,
    theme: {
        accent: "#c4b5fd",
        tile: "linear-gradient(180deg, #7c3aed 0%, #4c1d95 100%)",
        ink: "#ede9fe",
    },
    initialPosition: { x: 24, y: 68 },
};

export function DownloadModpacksWindow() {
    return (
        <WindowFrame metadata={downloadModpacksWindowMetadata}>
            <WindowEmptyState icon={<PackageIcon />} label="暂无整合包" />
        </WindowFrame>
    );
}
