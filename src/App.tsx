import { useEffect, useState } from 'react';

export function App() {
  const [windowLabel, setWindowLabel] = useState<string>('main');

  useEffect(() => {
    // Attempt to detect window label in Tauri context
    import('@tauri-apps/api/webviewWindow')
      .then(({ getCurrentWebviewWindow }) => {
        const appWindow = getCurrentWebviewWindow();
        setWindowLabel(appWindow.label);
      })
      .catch(() => {
        // Fallback for non-Tauri browser preview
      });
  }, []);

  return (
    <main className="min-h-screen bg-slate-900 text-white flex flex-col items-center justify-center p-4">
      <h1 className="text-2xl font-bold mb-2">GIF Recorder</h1>
      <p className="text-slate-400">Current Window: {windowLabel}</p>
    </main>
  );
}

export default App;
