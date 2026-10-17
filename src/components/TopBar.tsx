import { useEffect, useRef } from "react";
import { PlayerIcon } from "./PlayerIcon";
import type { Section } from "./AppSidebar";

type TopBarProps = { onHome: () => void; onBack: () => void; onForward: () => void; canGoBack: boolean; canGoForward: boolean; onOpenQueue: () => void; nowPlayingOpen: boolean; onToggleNowPlaying: () => void; activeSection: Section; search: string; onSearchChange: (value: string) => void };
export function TopBar({ activeSection, search, onSearchChange, onHome, onBack, onForward, canGoBack, canGoForward, onOpenQueue, nowPlayingOpen, onToggleNowPlaying }: TopBarProps) {
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    function focusSearch(event: KeyboardEvent) {
      const target = event.target;
      const typing = target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));
      if (event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "k" && !typing && !document.querySelector("dialog[open]")) {
        event.preventDefault(); input.current?.focus(); input.current?.select();
      }
    }
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  }, []);
  return <header className="topbar">
    <div className="app-brand"><span className="brand-mark" aria-hidden="true"><i /><i /><i /></span><span>afterhours</span></div>
    <nav className="top-navigation" aria-label="Browse history"><button className="icon-button" type="button" aria-label="Go back" title="Go back" disabled={!canGoBack} onClick={onBack}><PlayerIcon name="back" /></button><button className="icon-button" type="button" aria-label="Go forward" title="Go forward" disabled={!canGoForward} onClick={onForward}><PlayerIcon name="forward" /></button><button className="icon-button home-button" type="button" aria-label="Home" title="Home" aria-current={activeSection === "Home" && !search ? "page" : undefined} onClick={onHome}><PlayerIcon name="home" /></button></nav>
    <div className="global-search"><label className="search-field"><span className="sr-only">Search all local songs by title, artist, or album</span><PlayerIcon name="search" /><input ref={input} type="search" placeholder="What do you want to play?" title="Search local music (Ctrl+K when not typing)" value={search} onChange={event=>onSearchChange(event.currentTarget.value)} onKeyDown={event=>{if(event.key==="Escape"){event.stopPropagation();onSearchChange("");}}} /></label>{search?<button type="button" className="icon-button search-clear" aria-label="Clear search" title="Clear search" onClick={()=>{onSearchChange("");input.current?.focus();}}><PlayerIcon name="close" /></button>:<span className="search-shortcut" aria-hidden="true">Ctrl K</span>}</div>
    <div className="top-utilities"><span className="local-badge"><span />Local library</span><button className="icon-button" type="button" title="Queue" aria-label="Open queue" onClick={onOpenQueue}><PlayerIcon name="queue" /></button><button className="icon-button now-playing-toggle" type="button" title={nowPlayingOpen?"Hide Now Playing":"Show Now Playing"} aria-label={nowPlayingOpen?"Hide Now Playing":"Show Now Playing"} aria-pressed={nowPlayingOpen} onClick={onToggleNowPlaying}><PlayerIcon name="panel" /></button></div>
  </header>;
}
