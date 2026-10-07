import { useEffect, useState } from 'react';
import { RegionSelector } from './windows/RegionSelector';
import { LiveOverlay } from './windows/LiveOverlay';
import { StudioView } from './windows/StudioView';

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
    <StudioView
      onRecordAgain={handleOpenSelector}
      onOpenSelector={handleOpenSelector}
    />
  );
}

export default App;
