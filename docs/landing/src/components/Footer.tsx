import { useTranslation } from 'react-i18next';

export function Footer() {
  const { t } = useTranslation();

  return (
    <footer className="py-16 bg-brand-bg border-t border-white/5">
      <div className="container mx-auto px-4 flex flex-col md:flex-row items-center justify-between gap-4 text-sm text-gray-500">
        <div>
          {t('footer.copyright')}
        </div>
        <div className="flex gap-6">
          <a href="/privacy" className="hover:text-white transition-colors">{t('footer.privacy')}</a>
          <a href="/terms" className="hover:text-white transition-colors">{t('footer.terms')}</a>
        </div>
      </div>
    </footer>
  );
}
