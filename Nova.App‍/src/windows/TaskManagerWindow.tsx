import { LayersIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const taskManagerWindowMetadata: WindowMetadata = {
    id: "task-manager",
    title: "任务管理器",
    Icon: LayersIcon,
    theme: {
        accent: "#facc15",
        tile: "linear-gradient(180deg, #ca8a04 0%, #713f12 100%)",
        ink: "#fef9c3",
    },
    initialPosition: { x: 24, y: 124 },
};

export function TaskManagerWindow() {
    return (
        <WindowFrame metadata={taskManagerWindowMetadata}>
            <WindowEmptyState icon={<LayersIcon />} label="暂无任务" />
        </WindowFrame>
    );
}
