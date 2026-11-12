import { FileArchiveIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const downloadDataPacksWindowMetadata: WindowMetadata = {
    id: "download-data-packs",
    title: "下载数据包",
    Icon: FileArchiveIcon,
    theme: {
        accent: "#6ee7b7",
        tile: "linear-gradient(180deg, #059669 0%, #064e3b 100%)",
        ink: "#d1fae5",
    },
    initialPosition: { x: 24, y: 92 },
};

export function DownloadDataPacksWindow() {
    return (
        <WindowFrame metadata={downloadDataPacksWindowMetadata}>
            <WindowEmptyState icon={<FileArchiveIcon />} label="暂无数据包" />
        </WindowFrame>
    );
}
