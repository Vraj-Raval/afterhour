import type { Album } from "../data/home";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ArtworkRef } from "../library/types";

type ArtworkProps = { compact?: boolean } & ({ album: Album; artwork?: never } | { artwork: ArtworkRef | null; album?: never });

function LibraryArtwork({ artwork, compact }: { artwork: ArtworkRef | null; compact: boolean }) {
  const container = useRef<HTMLDivElement>(null);
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!artwork || !container.current) return;
    let disposed = false;
    let url: string | undefined;
    async function fetchArtwork() {
      try {
        const bytes = await invoke<ArrayBuffer>("get_music_artwork", { id: artwork!.id });
        if (disposed) return;
        url = URL.createObjectURL(new Blob([bytes], { type: artwork!.mime }));
        setSrc(url);
      } catch { if (!disposed) setFailed(true); }
    }
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) { observer.disconnect(); void fetchArtwork(); }
    });
    observer.observe(container.current);
    return () => { disposed = true; observer.disconnect(); if (url) URL.revokeObjectURL(url); };
  }, [artwork]);

  return <div ref={container} className={`artwork real-artwork${compact ? " artwork-compact" : ""}`} aria-hidden="true">
    {src && !failed ? <img src={src} alt="" onError={() => setFailed(true)} /> : <svg viewBox="0 0 48 48" fill="none" stroke="currentColor"><circle cx="24" cy="24" r="15" /><circle cx="24" cy="24" r="3" /><path d="M24 9a15 15 0 0 1 15 15" strokeWidth="3" /></svg>}
  </div>;
}

export function AlbumArtwork(props: ArtworkProps) {
  const { compact = false } = props;
  if (!props.album) return <LibraryArtwork key={props.artwork?.id ?? "fallback"} artwork={props.artwork ?? null} compact={compact} />;
  const album = props.album;
  return (
    <div className={`artwork album-art art-${album.artwork}${compact ? " artwork-compact" : ""}`} aria-hidden="true">
      <span className="art-shape" />
      {!compact && <><span className="art-title">{album.title}</span><span className="art-number">AFTERHOURS / {album.id}</span></>}
    </div>
  );
}
