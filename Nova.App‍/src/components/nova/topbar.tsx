import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { useWindowStore } from "@/stores/window-store";
import { WINDOW_METADATA } from "@/windows";

const partsFormatter = new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    weekday: "short",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
    timeZone: "Asia/Shanghai",
});

function formatNow(date: Date) {
    const get = (t: string) => partsFormatter.formatToParts(date).find((p) => p.type === t)?.value ?? "";
    return `${get("month")}月${get("day")}日 ${get("weekday")} ${get("hour")}:${get("minute")}`;
}

export function useNow(intervalMs = 30_000) {
    const [now, setNow] = useState(() => new Date());

    useEffect(() => {
        const id = setInterval(() => setNow(new Date()), intervalMs);
        return () => clearInterval(id);
    }, [intervalMs]);

    return now;
}

export function TopBar() {
    const now = useNow();
    const activeWindowId = useWindowStore((state) => state.activeWindowId);
    const activeWindow = WINDOW_METADATA.find((window) => window.id === activeWindowId);

    return (
        <nav className="flex h-8 items-center border-b border-accent/50 px-4 text-sm">
            {/* 左侧 */}
            <section className="flex items-center gap-4">
                <img src="/images/NovaText.svg" className="size-4" />
                <span className="font-bold">{activeWindow?.title ?? "Nova Launcher"}</span>
            </section>
            {/* 右侧 */}
            <section className="ml-auto flex items-center gap-4">
                <Badge variant="secondary" className="bg-violet-50 text-violet-700 dark:bg-violet-950 dark:text-violet-300">
                    INDEV
                </Badge>
                <span className="tabular-nums">{formatNow(now)}</span>
            </section>
        </nav>
    );
}
