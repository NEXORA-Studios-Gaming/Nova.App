import GradientWaves from "@/components/bits/GradientWaves";

export function AppBackground() {
    return (
        <GradientWaves
            horizonColor="#1e3a8a"
            waveColor="#38bdf8"
            crestColor="#e0f2fe"
            speed={0.5}
            amplitude={1}
            relief={1}
            lightAngle={180}
            mouseInteraction
            swellOnClick={false}
            waveScale={0.9}
            swell={25}
            turbulence={18}
            zoom={0.8}
            height={8}
            fogDepth={24}
            className="pointer-events-none! fixed! -z-100 blur-md"
        />
    );
}
