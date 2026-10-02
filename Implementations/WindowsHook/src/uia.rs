use windows::{
    core::BSTR,
    Win32::{
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        },
        UI::Accessibility::{
            CUIAutomation8, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
            IUIAutomationTextRange, IUIAutomationValuePattern, TextPatternRangeEndpoint_End,
            TextPatternRangeEndpoint_Start, TextUnit_Character, UIA_DocumentControlTypeId,
            UIA_EditControlTypeId, UIA_TextControlTypeId, UIA_TextPatternId, UIA_ValuePatternId,
        },
    },
};

#[derive(Debug)]
pub enum CaptureError {
    Unsupported,
    Denied,
}

pub struct AutomationText {
    automation: IUIAutomation,
    com_initialized: bool,
}

pub struct OwnedText {
    element: IUIAutomationElement,
    prefix: String,
    suffix: String,
    target: isize,
    process_id: u32,
}

impl OwnedText {
    pub fn target(&self) -> isize {
        self.target
    }
}

impl AutomationText {
    pub fn new() -> Result<Self, String> {
        unsafe {
            // The compatibility worker also owns OLE clipboard state. OleInitialize
            // selects a single-threaded apartment, so UI Automation must join the same
            // apartment instead of trying to switch the worker to MTA.
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|error| format!("CoInitializeEx failed: {error}"))?;
        }
        let automation = unsafe {
            CoCreateInstance::<_, IUIAutomation>(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| format!("CUIAutomation8 failed: {error}"))?
        };
        Ok(Self {
            automation,
            com_initialized: true,
        })
    }

    pub fn focused_keyboard_text_surface(&self, process_id: u32) -> Result<bool, CaptureError> {
        let element = unsafe {
            self.automation
                .GetFocusedElement()
                .map_err(|_| CaptureError::Unsupported)?
        };

        let actual_process = unsafe {
            element
                .CurrentProcessId()
                .map_err(|_| CaptureError::Unsupported)?
        };
        if actual_process != process_id as i32 {
            return Ok(false);
        }

        let has_focus = unsafe {
            element
                .CurrentHasKeyboardFocus()
                .map_err(|_| CaptureError::Unsupported)?
                .as_bool()
        };
        let keyboard_focusable = unsafe {
            element
                .CurrentIsKeyboardFocusable()
                .map_err(|_| CaptureError::Unsupported)?
                .as_bool()
        };
        let enabled = unsafe {
            element
                .CurrentIsEnabled()
                .map_err(|_| CaptureError::Unsupported)?
                .as_bool()
        };
        if !has_focus || !keyboard_focusable || !enabled {
            return Ok(false);
        }

        if unsafe {
            element
                .CurrentIsPassword()
                .map_err(|_| CaptureError::Unsupported)?
                .as_bool()
        } {
            return Err(CaptureError::Denied);
        }

        let control_type = unsafe {
            element
                .CurrentControlType()
                .map_err(|_| CaptureError::Unsupported)?
        };
        if control_type != UIA_EditControlTypeId
            && control_type != UIA_TextControlTypeId
            && control_type != UIA_DocumentControlTypeId
        {
            return Ok(false);
        }

        let _: IUIAutomationTextPattern = unsafe {
            element
                .GetCurrentPatternAs(UIA_TextPatternId)
                .map_err(|_| CaptureError::Unsupported)?
        };
        Ok(true)
    }

    pub fn capture_focused(
        &self,
        target: isize,
        process_id: u32,
    ) -> Result<OwnedText, CaptureError> {
        let element = unsafe {
            self.automation
                .GetFocusedElement()
                .map_err(|_| CaptureError::Unsupported)?
        };

        let actual_process = unsafe {
            element
                .CurrentProcessId()
                .map_err(|_| CaptureError::Unsupported)?
        };
        if actual_process != process_id as i32 {
            return Err(CaptureError::Unsupported);
        }

        if !unsafe {
            element
                .CurrentHasKeyboardFocus()
                .map_err(|_| CaptureError::Unsupported)?
                .as_bool()
        } {
            return Err(CaptureError::Unsupported);
        }

        if unsafe {
            element
                .CurrentIsPassword()
                .map_err(|_| CaptureError::Unsupported)?
                .as_bool()
        } {
            return Err(CaptureError::Denied);
        }

        let control_type = unsafe {
            element
                .CurrentControlType()
                .map_err(|_| CaptureError::Unsupported)?
        };
        if control_type != UIA_EditControlTypeId {
            return Err(CaptureError::Unsupported);
        }

        let value: IUIAutomationValuePattern = unsafe {
            element
                .GetCurrentPatternAs(UIA_ValuePatternId)
                .map_err(|_| CaptureError::Unsupported)?
        };
        if unsafe {
            value
                .CurrentIsReadOnly()
                .map_err(|_| CaptureError::Unsupported)?
                .as_bool()
        } {
            return Err(CaptureError::Denied);
        }

        let text: IUIAutomationTextPattern = unsafe {
            element
                .GetCurrentPatternAs(UIA_TextPatternId)
                .map_err(|_| CaptureError::Unsupported)?
        };
        let selections = unsafe { text.GetSelection().map_err(|_| CaptureError::Unsupported)? };
        if unsafe { selections.Length().unwrap_or_default() } != 1 {
            return Err(CaptureError::Unsupported);
        }
        let selection = unsafe {
            selections
                .GetElement(0)
                .map_err(|_| CaptureError::Unsupported)?
        };
        let document = unsafe {
            text.DocumentRange()
                .map_err(|_| CaptureError::Unsupported)?
        };

        let prefix_range = unsafe { document.Clone().map_err(|_| CaptureError::Unsupported)? };
        unsafe {
            prefix_range
                .MoveEndpointByRange(
                    TextPatternRangeEndpoint_End,
                    &selection,
                    TextPatternRangeEndpoint_Start,
                )
                .map_err(|_| CaptureError::Unsupported)?;
        }
        let prefix = unsafe {
            prefix_range
                .GetText(-1)
                .map_err(|_| CaptureError::Unsupported)?
                .to_string()
        };

        let suffix_range = unsafe { document.Clone().map_err(|_| CaptureError::Unsupported)? };
        unsafe {
            suffix_range
                .MoveEndpointByRange(
                    TextPatternRangeEndpoint_Start,
                    &selection,
                    TextPatternRangeEndpoint_End,
                )
                .map_err(|_| CaptureError::Unsupported)?;
        }
        let suffix = unsafe {
            suffix_range
                .GetText(-1)
                .map_err(|_| CaptureError::Unsupported)?
                .to_string()
        };

        Ok(OwnedText {
            element,
            prefix,
            suffix,
            target,
            process_id,
        })
    }

    pub fn replace(&self, owned: &OwnedText, text: &str) -> Result<(), String> {
        let actual_process = unsafe {
            owned
                .element
                .CurrentProcessId()
                .map_err(|error| format!("CurrentProcessId failed: {error}"))?
        };
        if actual_process != owned.process_id as i32 {
            return Err("UI Automation control process changed".into());
        }
        if !unsafe {
            owned
                .element
                .CurrentHasKeyboardFocus()
                .map_err(|error| format!("CurrentHasKeyboardFocus failed: {error}"))?
                .as_bool()
        } {
            return Err("UI Automation control no longer has keyboard focus".into());
        }

        let value: IUIAutomationValuePattern = unsafe {
            owned
                .element
                .GetCurrentPatternAs(UIA_ValuePatternId)
                .map_err(|error| format!("ValuePattern unavailable: {error}"))?
        };
        let next = format!("{}{}{}", owned.prefix, text, owned.suffix);
        unsafe {
            value
                .SetValue(&BSTR::from(next.as_str()))
                .map_err(|error| format!("SetValue failed: {error}"))?;
        }

        let actual = unsafe {
            value
                .CurrentValue()
                .map_err(|error| format!("CurrentValue failed: {error}"))?
                .to_string()
        };
        if actual != next {
            return Err("UI Automation control did not accept the requested text".into());
        }

        // At this point the requested value is already verified in the target.
        // Caret placement is a secondary refinement: failing it must never make the
        // caller replay the physical key, otherwise one keystroke is applied twice.
        let _ = self.place_caret(
            &owned.element,
            owned.prefix.chars().count() + text.chars().count(),
        );
        Ok(())
    }

    fn place_caret(&self, element: &IUIAutomationElement, offset: usize) -> Result<(), String> {
        let pattern: IUIAutomationTextPattern = unsafe {
            element
                .GetCurrentPatternAs(UIA_TextPatternId)
                .map_err(|error| format!("TextPattern unavailable: {error}"))?
        };
        let range: IUIAutomationTextRange = unsafe {
            pattern
                .DocumentRange()
                .map_err(|error| format!("DocumentRange failed: {error}"))?
        };
        let moved = unsafe {
            range
                .MoveEndpointByUnit(
                    TextPatternRangeEndpoint_Start,
                    TextUnit_Character,
                    offset as i32,
                )
                .map_err(|error| format!("MoveEndpointByUnit(caret) failed: {error}"))?
        };
        if moved < offset as i32 {
            return Err("could not place caret at requested position".into());
        }
        // Do not use the same COM range as both destination and source.
        // Some XAML/RichEdit providers (notably Windows Search) reject or mis-handle
        // that self-referential collapse.
        let anchor = unsafe {
            range
                .Clone()
                .map_err(|error| format!("Clone(caret anchor) failed: {error}"))?
        };
        unsafe {
            range
                .MoveEndpointByRange(
                    TextPatternRangeEndpoint_End,
                    &anchor,
                    TextPatternRangeEndpoint_Start,
                )
                .map_err(|error| format!("Collapse caret range failed: {error}"))?;
            range
                .Select()
                .map_err(|error| format!("Select(caret) failed: {error}"))?;
        }
        Ok(())
    }
}

impl Drop for AutomationText {
    fn drop(&mut self) {
        if self.com_initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputkey_windows_clipboard::ClipboardPaste;

    #[test]
    fn automation_joins_the_worker_ole_apartment() {
        let _clipboard = ClipboardPaste::new().expect("OLE clipboard apartment");
        let _automation = AutomationText::new().expect("UI Automation on the same STA worker");
    }
}
