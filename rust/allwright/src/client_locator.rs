use super::selectors::chain_selector_for_transport;
use super::types::{
    ClickResult, CommandOptions, CountResult, ElementResult, FillResult, HighlightOptions,
    HighlightResult, Locator, Page, PressOptions, PressResult, Result, TextResult,
    WaitForSelectorOptions, WaitForSelectorResult,
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

    pub async fn click(&self) -> Result<ClickResult> {
        self.page.click(self.selector.clone()).await
    }

    pub async fn click_with_options(&self, options: CommandOptions) -> Result<ClickResult> {
        self.page
            .click_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn count(&self) -> Result<CountResult> {
        self.page.count(self.selector.clone()).await
    }

    pub async fn count_with_options(&self, options: CommandOptions) -> Result<CountResult> {
        self.page
            .count_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn highlight(&self) -> Result<HighlightResult> {
        self.page.highlight(self.selector.clone()).await
    }

    pub async fn highlight_with_options(
        &self,
        options: HighlightOptions,
    ) -> Result<HighlightResult> {
        self.page
            .highlight_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn focus(&self) -> Result<ElementResult> {
        self.page.focus(self.selector.clone()).await
    }

    pub async fn focus_with_options(&self, options: CommandOptions) -> Result<ElementResult> {
        self.page
            .focus_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn fill(&self, value: impl Into<String>) -> Result<FillResult> {
        self.page.fill(self.selector.clone(), value.into()).await
    }

    pub async fn fill_with_options(
        &self,
        value: impl Into<String>,
        options: CommandOptions,
    ) -> Result<FillResult> {
        self.page
            .fill_with_options(self.selector.clone(), value.into(), options)
            .await
    }

    pub async fn hover(&self) -> Result<ElementResult> {
        self.page.hover(self.selector.clone()).await
    }

    pub async fn hover_with_options(&self, options: CommandOptions) -> Result<ElementResult> {
        self.page
            .hover_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn press(&self, key: impl Into<String>) -> Result<PressResult> {
        self.page.press(self.selector.clone(), key.into()).await
    }

    pub async fn press_with_options(
        &self,
        key: impl Into<String>,
        options: PressOptions,
    ) -> Result<PressResult> {
        self.page
            .press_with_options(self.selector.clone(), key.into(), options)
            .await
    }

    pub async fn text_content(&self) -> Result<TextResult> {
        self.page.text_content(self.selector.clone()).await
    }

    pub async fn text_content_with_options(&self, options: CommandOptions) -> Result<TextResult> {
        self.page
            .text_content_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn inner_text(&self) -> Result<TextResult> {
        self.page.inner_text(self.selector.clone()).await
    }

    pub async fn inner_text_with_options(&self, options: CommandOptions) -> Result<TextResult> {
        self.page
            .inner_text_with_options(self.selector.clone(), options)
            .await
    }

    pub async fn wait_for(&self) -> Result<WaitForSelectorResult> {
        self.wait_for_with_options(WaitForSelectorOptions {
            visible: Some(true),
            ..Default::default()
        })
        .await
    }

    pub async fn wait_for_with_options(
        &self,
        options: WaitForSelectorOptions,
    ) -> Result<WaitForSelectorResult> {
        let mut options = options;
        if options.visible.is_none() {
            options.visible = Some(true);
        }
        self.page
            .wait_for_selector_with_options(self.selector.clone(), options)
            .await
    }
}
