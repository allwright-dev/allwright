use super::selectors::chain_selector_for_transport;
use super::types::{
    CommandOptions, HighlightOptions, Locator, Page, PressOptions, Result, WaitForSelectorOptions,
};

impl Locator {
    pub fn page(&self) -> &Page {
        &self.page
    }

    pub fn selector(&self) -> &str {
        &self.selector
    }

    pub fn locator(&self, css_selector: impl Into<String>) -> Locator {
        let child_selector = css_selector.into();
        Locator {
            page: self.page.clone(),
            selector: chain_selector_for_transport(&self.selector, &child_selector),
        }
    }

    pub async fn frame(&self) -> Result<Page> {
        self.frame_with_options(CommandOptions::default()).await
    }

    pub async fn frame_with_options(&self, options: CommandOptions) -> Result<Page> {
        self.page
            .frame_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn click(&self) -> Result<()> {
        self.page.click(self.selector.clone()).await
    }

    pub async fn click_with_options(&self, options: CommandOptions) -> Result<()> {
        self.page
            .click_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn count(&self) -> Result<u32> {
        self.page.count(self.selector.clone()).await
    }

    pub async fn count_with_options(&self, options: CommandOptions) -> Result<u32> {
        self.page
            .count_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn highlight(&self) -> Result<()> {
        self.page.highlight(self.selector.clone()).await
    }

    pub async fn highlight_with_options(&self, options: HighlightOptions) -> Result<()> {
        self.page
            .highlight_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn focus(&self) -> Result<()> {
        self.page.focus(self.selector.clone()).await
    }

    pub async fn focus_with_options(&self, options: CommandOptions) -> Result<()> {
        self.page
            .focus_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn fill(&self, value: impl Into<String>) -> Result<()> {
        self.page.fill(self.selector.clone(), value.into()).await
    }

    pub async fn fill_with_options(
        &self,
        value: impl Into<String>,
        options: CommandOptions,
    ) -> Result<()> {
        self.page
            .fill_with_options(self.selector.clone(), value.into(), options)
            .await
    }

    pub async fn hover(&self) -> Result<()> {
        self.page.hover(self.selector.clone()).await
    }

    pub async fn hover_with_options(&self, options: CommandOptions) -> Result<()> {
        self.page
            .hover_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn press(&self, key: impl Into<String>) -> Result<()> {
        self.page.press(self.selector.clone(), key.into()).await
    }

    pub async fn press_with_options(
        &self,
        key: impl Into<String>,
        options: PressOptions,
    ) -> Result<()> {
        self.page
            .press_with_options(self.selector.clone(), key.into(), options)
            .await
    }

    pub async fn text_content(&self) -> Result<Option<String>> {
        self.page.text_content(self.selector.clone()).await
    }

    pub async fn text_content_with_options(
        &self,
        options: CommandOptions,
    ) -> Result<Option<String>> {
        self.page
            .text_content_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn inner_text(&self) -> Result<String> {
        self.page.inner_text(self.selector.clone()).await
    }

    pub async fn inner_text_with_options(&self, options: CommandOptions) -> Result<String> {
        self.page
            .inner_text_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn wait_for(&self) -> Result<()> {
        self.wait_for_with_options(WaitForSelectorOptions {
            visible: Some(true),
            ..Default::default()
        })
        .await
    }

    pub async fn wait_for_with_options(&self, options: WaitForSelectorOptions) -> Result<()> {
        let mut options = options;
        if options.visible.is_none() {
            options.visible = Some(true);
        }
        self.page
            .wait_for_selector_with_options(self.selector.clone(), options)
            .await
    }
}
