import { HomeIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const homeWindowMetadata: WindowMetadata = {
    id: "home",
    title: "主页",
    Icon: HomeIcon,
    theme: {
        accent: "#38bdf8",
        tile: "linear-gradient(180deg, #0e7490 0%, #164e63 100%)",
        ink: "#e0f2fe",
    },
    initialPosition: { x: 24, y: 36 },
    initiallyOpen: true,
};

export function HomeWindow() {
    return (
        <WindowFrame metadata={homeWindowMetadata}>
            <div className="flex items-center justify-between gap-4">
                <div className="flex min-w-0 items-center gap-3">
                    <img src="/images/NovaText.svg" alt="" className="size-9 shrink-0" draggable={false} />
                    <div className="min-w-0">
                        <p className="truncate text-sm font-semibold">Nova Launcher</p>
                        <p className="truncate text-xs text-muted-foreground">Project NOVA</p>
                    </div>
                </div>
                <Badge variant="secondary">INDEV</Badge>
            </div>
        </WindowFrame>
    );
}
