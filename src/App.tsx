import { useEffect, useState } from 'react';
import { RegionSelector } from './windows/RegionSelector';
import { LiveOverlay } from './windows/LiveOverlay';
import { Video, Crop, Sparkles, Layers } from 'lucide-react';

export function App() {
  const [windowLabel, setWindowLabel] = useState<string>(() => {
    // Check URL query param first (e.g. ?window=selector or ?window=overlay)
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      const queryWindow = params.get('window');
      if (queryWindow) return queryWindow;

      // Check URL hash fallback (e.g. #selector or #overlay)
      const hash = window.location.hash.replace(/^#\/?/, '').toLowerCase();
      if (hash === 'selector' || hash === 'overlay' || hash === 'main') {
        return hash;
      }
    }
    return 'main';
  });

  useEffect(() => {
    // Listen to hash changes in browser preview
    const handleHashChange = () => {
      const hash = window.location.hash.replace(/^#\/?/, '').toLowerCase();
      if (hash === 'selector' || hash === 'overlay' || hash === 'main') {
        setWindowLabel(hash);
      }
    };
    window.addEventListener('hashchange', handleHashChange);

    // Detect window label in native Tauri context
    import('@tauri-apps/api/webviewWindow')
      .then(({ getCurrentWebviewWindow }) => {
        const appWindow = getCurrentWebviewWindow();
        if (appWindow?.label) {
          setWindowLabel(appWindow.label);
        }
      })
      .catch(() => {
        // Running in non-Tauri preview environment
      });

    return () => window.removeEventListener('hashchange', handleHashChange);
  }, []);

  const handleOpenSelector = async () => {
    try {
      const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      const selectorWin = await WebviewWindow.getByLabel('selector');
      if (selectorWin) {
        await selectorWin.show();
        await selectorWin.setFocus();
      } else {
        setWindowLabel('selector');
      }
    } catch {
      setWindowLabel('selector');
    }
  };

  const handleOpenOverlayPreview = () => {
    setWindowLabel('overlay');
  };

  // Route to RegionSelector window
  if (windowLabel === 'selector') {
    return (
      <RegionSelector
        onCancel={() => {
          import('@tauri-apps/api/webviewWindow')
            .then(({ getCurrentWebviewWindow }) => getCurrentWebviewWindow().hide())
            .catch(() => setWindowLabel('main'));
        }}
        onConfirm={(region) => {
          import('@tauri-apps/api/core')
            .then(({ invoke }) => {
              invoke('start_recording', {
                region: {
                  x: Math.round(region.x),
                  y: Math.round(region.y),
                  width: Math.round(region.width),
                  height: Math.round(region.height),
                },
              }).catch((e) => console.error('Failed to start recording:', e));
            })
            .catch(() => {
              // Fallback preview
              setWindowLabel('overlay');
            });
        }}
      />
    );
  }

  // Route to LiveOverlay window
  if (windowLabel === 'overlay') {
    return (
      <LiveOverlay
        onStop={() => {
          import('@tauri-apps/api/core')
            .then(({ invoke }) => {
              invoke('stop_recording').catch(() => {});
            })
            .catch(() => {
              setWindowLabel('main');
            });
        }}
      />
    );
  }

  // Route to Main Studio window
  return (
    <main className="min-h-screen bg-slate-950 text-slate-100 flex flex-col items-center justify-center p-6 selection:bg-blue-500 selection:text-white">
      <div className="w-full max-w-lg bg-slate-900/90 border border-slate-800 rounded-2xl p-8 shadow-2xl backdrop-blur-xl">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2.5 bg-red-600/20 text-red-400 rounded-xl border border-red-500/30">
            <Video className="w-6 h-6" />
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight text-white">GIF Recorder Studio</h1>
            <p className="text-xs text-slate-400">Capture, live annotate, and produce crisp GIFs</p>
          </div>
        </div>

        <div className="space-y-3 my-6">
          <button
            type="button"
            onClick={handleOpenSelector}
            className="w-full flex items-center justify-between px-4 py-3 bg-red-600 hover:bg-red-500 active:bg-red-700 text-white font-medium rounded-xl transition shadow-lg shadow-red-950/50 cursor-pointer"
          >
            <span className="flex items-center gap-2.5">
              <Crop className="w-4 h-4" /> Select Region to Record
            </span>
            <span className="text-xs bg-red-700/60 px-2 py-0.5 rounded font-mono">Drag Marquee</span>
          </button>

          <button
            type="button"
            onClick={handleOpenOverlayPreview}
            className="w-full flex items-center justify-between px-4 py-3 bg-slate-800 hover:bg-slate-700 active:bg-slate-750 text-slate-200 font-medium rounded-xl border border-slate-700 transition cursor-pointer"
          >
            <span className="flex items-center gap-2.5">
              <Layers className="w-4 h-4 text-blue-400" /> Preview Live Overlay
            </span>
            <span className="text-xs text-slate-400 flex items-center gap-1">
              <Sparkles className="w-3 h-3 text-purple-400" /> Presenter
            </span>
          </button>
        </div>

        <div className="pt-4 border-t border-slate-800 flex items-center justify-between text-xs text-slate-500">
          <span>Active Window: <code className="text-slate-400">{windowLabel}</code></span>
          <div className="flex gap-2">
            <a href="?window=selector" className="hover:text-slate-300">#selector</a>
            <span>•</span>
            <a href="?window=overlay" className="hover:text-slate-300">#overlay</a>
          </div>
        </div>
      </div>
    </main>
  );
}

export default App;
