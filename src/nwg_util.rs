use native_windows_gui as nwg;

pub trait SaneBuilder: Sized {
    type Built: Default;
    fn build(self, obj: &mut Self::Built) -> Result<(), nwg::NwgError>;
    fn construct(self) -> Result<Self::Built, nwg::NwgError> {
        let mut obj = Default::default();
        self.build(&mut obj)?;
        Ok(obj)
    }
}

macro_rules! sane_builder {
    ($builder:ty, $built:ty) => {
        impl SaneBuilder for $builder {
            type Built = $built;
            fn build(self, obj: &mut Self::Built) -> Result<(), nwg::NwgError> {
                self.build(obj)
            }
        }
    };
}

sane_builder!(nwg::ButtonBuilder<'_>, nwg::Button);
sane_builder!(nwg::WindowBuilder<'_>, nwg::Window);
sane_builder!(nwg::RadioButtonBuilder<'_>, nwg::RadioButton);
sane_builder!(nwg::ExternCanvasBuilder<'_>, nwg::ExternCanvas);
sane_builder!(nwg::FontBuilder<'_>, nwg::Font);
sane_builder!(nwg::TrackBarBuilder, nwg::TrackBar);
sane_builder!(nwg::ColorDialogBuilder, nwg::ColorDialog);
