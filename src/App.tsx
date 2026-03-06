import { useState, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import FloatingPanel from './components/FloatingPanel';
import Settings from './components/Settings';

function App() {
  const [windowLabel, setWindowLabel] = useState<string>('');

  useEffect(() => {
    const label = getCurrentWindow().label;
    setWindowLabel(label);
  }, []);

  // Wait until label is resolved to avoid flash
  if (!windowLabel) return null;

  if (windowLabel === 'settings') {
    return <Settings />;
  }

  return <FloatingPanel />;
}

export default App;
