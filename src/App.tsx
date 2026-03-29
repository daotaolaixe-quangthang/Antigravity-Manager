import { createBrowserRouter, RouterProvider } from 'react-router-dom';

import Layout from './components/layout/Layout';
import Dashboard from './pages/Dashboard';
import Accounts from './pages/Accounts';
import Settings from './pages/Settings';
import ApiProxy from './pages/ApiProxy';
import Monitor from './pages/Monitor';
import TokenStats from './pages/TokenStats';
import Security from './pages/Security';
import ThemeManager from './components/common/ThemeManager';
import UserToken from './pages/UserToken';
import { UpdateNotification } from './components/UpdateNotification';
import DebugConsole from './components/debug/DebugConsole';
import { useEffect, useState } from 'react';
import { useConfigStore } from './stores/useConfigStore';
import { useAccountStore } from './stores/useAccountStore';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { isTauri } from './utils/env';
import { request as invoke } from './utils/request';
import { AdminAuthGuard } from './components/common/AdminAuthGuard';
import { showToast } from './components/common/ToastContainer';
import RotationSuggestionDialog, { RotationSuggestion } from './components/rotation/RotationSuggestionDialog';

const router = createBrowserRouter([
  {
    path: '/',
    element: <Layout />,
    children: [
      {
        index: true,
        element: <Dashboard />,
      },
      {
        path: 'accounts',
        element: <Accounts />,
      },
      {
        path: 'api-proxy',
        element: <ApiProxy />,
      },
      {
        path: 'monitor',
        element: <Monitor />,
      },
      {
        path: 'token-stats',
        element: <TokenStats />,
      },
      {
        path: 'user-token',
        element: <UserToken />,
      },
      {
        path: 'security',
        element: <Security />,
      },
      {
        path: 'settings',
        element: <Settings />,
      },
    ],
  },
]);

function App() {
  const { config, loadConfig } = useConfigStore();
  const { fetchCurrentAccount, fetchAccounts } = useAccountStore();
  const { i18n } = useTranslation();
  const [rotationSuggestion, setRotationSuggestion] = useState<RotationSuggestion | null>(null);
  const [rotationSwitching, setRotationSwitching] = useState(false);

  const syncRotationStatus = async () => {
    if (!isTauri()) return;
    try {
      const status = await invoke<{
        enabled: boolean;
        suggestion?: RotationSuggestion | null;
        is_switching: boolean;
        last_evaluated_at?: number | null;
      }>('get_rotation_status');
      setRotationSuggestion(status?.suggestion || null);
      setRotationSwitching(Boolean(status?.is_switching));
    } catch {
    }
  };

  const evaluateRotationNow = async () => {
    if (!isTauri()) return;
    try {
      const status = await invoke<{
        enabled: boolean;
        suggestion?: RotationSuggestion | null;
        is_switching: boolean;
        last_evaluated_at?: number | null;
      }>('evaluate_rotation_now');
      setRotationSuggestion(status?.suggestion || null);
      setRotationSwitching(Boolean(status?.is_switching));
    } catch {
    }
  };

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // Sync language from config
  useEffect(() => {
    if (config?.language) {
      i18n.changeLanguage(config.language);
      // Support RTL
      if (config.language === 'ar') {
        document.documentElement.dir = 'rtl';
      } else {
        document.documentElement.dir = 'ltr';
      }
    }
  }, [config?.language, i18n]);

  // Listen for tray events
  useEffect(() => {
    if (!isTauri()) return;
    const unlistenPromises: Promise<() => void>[] = [];

    // 监听托盘切换账号事件
    unlistenPromises.push(
      listen('tray://account-switched', () => {
        console.log('[App] Tray account switched, refreshing...');
        fetchCurrentAccount();
        fetchAccounts();
      })
    );

    // 监听托盘刷新事件
    unlistenPromises.push(
      listen('tray://refresh-current', () => {
        console.log('[App] Tray refresh triggered, refreshing...');
        fetchCurrentAccount();
        fetchAccounts();
      })
    );

    // 监听后端全量刷新事件 (Command / Scheduler)
    unlistenPromises.push(
      listen('accounts://refreshed', () => {
        console.log('[App] Backend triggered quota refresh, syncing UI...');
        fetchCurrentAccount();
        fetchAccounts();
      })
    );

    unlistenPromises.push(
      listen<RotationSuggestion>('rotation://suggested', (event) => {
        setRotationSuggestion(event.payload);
        if (config?.rotation?.notification_channels?.popup !== false) {
          showToast(
            `Rotation suggested: ${event.payload.current_account_email} -> ${event.payload.candidate.email}`,
            'warning',
            5000
          );
        }
      })
    );

    unlistenPromises.push(
      listen('rotation://dismissed', () => {
        setRotationSuggestion(null);
      })
    );

    unlistenPromises.push(
      listen<RotationSuggestion>('rotation://executed', (event) => {
        setRotationSuggestion(null);
        setRotationSwitching(false);
        fetchCurrentAccount();
        fetchAccounts();
        showToast(`Switched to ${event.payload.candidate.email}`, 'success');
      })
    );

    unlistenPromises.push(
      listen('rotation://focus-suggestion', () => {
        syncRotationStatus();
      })
    );

    unlistenPromises.push(
      listen('rotation://refresh-hint', () => {
        evaluateRotationNow();
      })
    );

    // Cleanup
    return () => {
      Promise.all(unlistenPromises).then(unlisteners => {
        unlisteners.forEach(unlisten => unlisten());
      });
    };
  }, [config?.rotation?.notification_channels?.popup, fetchCurrentAccount, fetchAccounts]);

  useEffect(() => {
    if (!isTauri() || !config?.rotation?.enabled) return;
    syncRotationStatus();
  }, [config?.rotation?.enabled]);

  // Update notification state
  const [showUpdateNotification, setShowUpdateNotification] = useState(false);

  // Check for updates on startup
  useEffect(() => {
    const checkUpdates = async () => {
      try {
        console.log('[App] Checking if we should check for updates...');
        const shouldCheck = await invoke<boolean>('should_check_updates');
        console.log('[App] Should check updates:', shouldCheck);

        if (shouldCheck) {
          setShowUpdateNotification(true);
          // 我们这里只负责显示通知组件，通知组件内部会去调用 check_for_updates
          // 我们在显示组件后，标记已经检查过了（即便失败或无更新，组件内部也会处理）
          await invoke('update_last_check_time');
          console.log('[App] Update check cycle initiated and last check time updated.');
        }
      } catch (error) {
        console.error('Failed to check update settings:', error);
      }
    };

    // Delay check to avoid blocking initial render
    const timer = setTimeout(checkUpdates, 2000);
    return () => clearTimeout(timer);
  }, []);

  const handleRotationSwitch = async () => {
    if (!rotationSuggestion || rotationSwitching) return;
    setRotationSwitching(true);
    try {
      await invoke('execute_rotation_switch');
    } catch (error) {
      setRotationSwitching(false);
      showToast(`Rotation switch failed: ${error}`, 'error');
    }
  };

  const handleRotationDismiss = async (remindAfterSeconds?: number) => {
    try {
      await invoke('dismiss_rotation_suggestion', { remindAfterSeconds });
      setRotationSuggestion(null);
    } catch (error) {
      showToast(`Failed to dismiss suggestion: ${error}`, 'error');
    }
  };

  return (
    <AdminAuthGuard>
      <ThemeManager />
      <DebugConsole />
      <RotationSuggestionDialog
        suggestion={config?.rotation?.notification_channels?.popup === false ? null : rotationSuggestion}
        switching={rotationSwitching}
        onSwitch={handleRotationSwitch}
        onDismiss={() => handleRotationDismiss(config?.rotation?.cooldown_seconds)}
        onRemindLater={() => handleRotationDismiss(300)}
      />
      {showUpdateNotification && (
        <UpdateNotification onClose={() => setShowUpdateNotification(false)} />
      )}
      <RouterProvider router={router} />
    </AdminAuthGuard>
  );
}

export default App;
