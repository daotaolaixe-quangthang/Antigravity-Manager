import { RefreshCw, Zap } from 'lucide-react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';

export interface RotationSuggestion {
    current_account_id: string;
    current_account_email: string;
    current_quota_percentage?: number | null;
    threshold_percentage: number;
    target_models: string[];
    candidate: {
        account_id: string;
        email: string;
        quota_percentage: number;
        reset_at?: number | null;
    };
    reason: {
        code: string;
        summary: string;
        detail: string;
    };
    created_at: number;
}

interface RotationSuggestionDialogProps {
    suggestion: RotationSuggestion | null;
    switching: boolean;
    onSwitch: () => void;
    onDismiss: () => void;
    onRemindLater: () => void;
}

export default function RotationSuggestionDialog({
    suggestion,
    switching,
    onSwitch,
    onDismiss,
    onRemindLater,
}: RotationSuggestionDialogProps) {
    const { t } = useTranslation();

    if (!suggestion) return null;

    return createPortal(
        <div className="modal modal-open z-[120]">
            <div data-tauri-drag-region className="fixed top-0 left-0 right-0 h-8 z-[130]" />
            <div className="modal-box relative max-w-lg bg-white dark:bg-base-100 shadow-2xl rounded-2xl p-0 overflow-hidden">
                <div className="p-6">
                    <div className="flex items-start gap-4">
                        <div className="w-12 h-12 rounded-2xl bg-amber-50 dark:bg-amber-900/20 flex items-center justify-center text-amber-500 shrink-0">
                            <Zap className="w-6 h-6" />
                        </div>
                        <div className="flex-1">
                            <h3 className="text-xl font-bold text-gray-900 dark:text-base-content">
                                {t('rotation.title', { defaultValue: 'Account rotation suggested' })}
                            </h3>
                            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                {suggestion.reason.summary}
                            </p>
                        </div>
                    </div>

                    <div className="mt-5 space-y-4">
                        <div className="rounded-xl border border-gray-100 dark:border-base-200 bg-gray-50 dark:bg-base-200 p-4">
                            <div className="text-xs font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                                {t('rotation.current', { defaultValue: 'Current account' })}
                            </div>
                            <div className="mt-2 text-sm font-semibold text-gray-900 dark:text-base-content">
                                {suggestion.current_account_email}
                            </div>
                            <div className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                                {suggestion.current_quota_percentage != null
                                    ? t('rotation.current_quota', {
                                        defaultValue: 'Target quota: {{quota}}% (threshold {{threshold}}%)',
                                        quota: suggestion.current_quota_percentage,
                                        threshold: suggestion.threshold_percentage,
                                    })
                                    : suggestion.reason.detail}
                            </div>
                        </div>

                        <div className="rounded-xl border border-emerald-100 dark:border-emerald-900/30 bg-emerald-50/70 dark:bg-emerald-900/10 p-4">
                            <div className="text-xs font-bold uppercase tracking-wider text-emerald-700 dark:text-emerald-400">
                                {t('rotation.candidate', { defaultValue: 'Suggested account' })}
                            </div>
                            <div className="mt-2 text-sm font-semibold text-gray-900 dark:text-base-content">
                                {suggestion.candidate.email}
                            </div>
                            <div className="mt-1 text-xs text-emerald-700 dark:text-emerald-400">
                                {t('rotation.candidate_quota', {
                                    defaultValue: 'Target quota available: {{quota}}%',
                                    quota: suggestion.candidate.quota_percentage,
                                })}
                            </div>
                        </div>

                        <div className="text-xs text-gray-500 dark:text-gray-400 leading-relaxed">
                            {t('rotation.targets', {
                                defaultValue: 'Target models: {{models}}',
                                models: suggestion.target_models.join(', '),
                            })}
                        </div>
                    </div>

                    <div className="flex gap-3 w-full mt-6">
                        <button
                            className="flex-1 px-4 py-2.5 bg-gray-100 dark:bg-base-200 text-gray-700 dark:text-gray-300 font-medium rounded-xl hover:bg-gray-200 dark:hover:bg-base-300 transition-colors"
                            onClick={onDismiss}
                            disabled={switching}
                        >
                            {t('rotation.dismiss', { defaultValue: 'Dismiss' })}
                        </button>
                        <button
                            className="flex-1 px-4 py-2.5 bg-white dark:bg-base-200 border border-gray-200 dark:border-base-300 text-gray-700 dark:text-gray-300 font-medium rounded-xl hover:bg-gray-50 dark:hover:bg-base-300 transition-colors"
                            onClick={onRemindLater}
                            disabled={switching}
                        >
                            {t('rotation.remind_later', { defaultValue: 'Remind later' })}
                        </button>
                        <button
                            className="flex-1 px-4 py-2.5 bg-blue-500 hover:bg-blue-600 text-white font-medium rounded-xl shadow-md transition-all flex items-center justify-center gap-2 disabled:opacity-70 disabled:cursor-not-allowed"
                            onClick={onSwitch}
                            disabled={switching}
                        >
                            {switching && <RefreshCw className="w-4 h-4 animate-spin" />}
                            {t('rotation.switch_now', { defaultValue: 'Switch now' })}
                        </button>
                    </div>
                </div>
            </div>
            <div className="modal-backdrop bg-black/40 backdrop-blur-sm fixed inset-0 z-[-1]"></div>
        </div>,
        document.body
    );
}
