#include "UISystemOption.h"

#include "../KeyAction.h"
#include "../Window.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Util/NxFiles.h"

namespace ms
{
	namespace
	{
		// Measured from the background. Each slider fills the gap between its
		// row's end labels, with the thumb centred on the row.
		struct SliderPlace
		{
			Point<int16_t> position;
			int16_t width;
			int16_t steps;
		};

		constexpr SliderPlace QUALITY_SLIDER = { Point<int16_t>(97, 39), 152, 2 };
		constexpr SliderPlace BGM_SLIDER = { Point<int16_t>(91, 69), 118, 10 };
		constexpr SliderPlace SFX_SLIDER = { Point<int16_t>(91, 99), 118, 10 };
		constexpr SliderPlace MOUSE_SLIDER = { Point<int16_t>(91, 159), 157, 10 };
		constexpr SliderPlace HP_SLIDER = { Point<int16_t>(94, 189), 155, 10 };
		constexpr SliderPlace MP_SLIDER = { Point<int16_t>(94, 219), 155, 10 };

		// Each volume and alert step is 10 of 100.
		constexpr int16_t PERCENT_STEP = 10;

		// The mute boxes stand right of the divider, before MUTE. The other
		// boxes stand before their text.
		constexpr Point<int16_t> BGM_MUTE_BOX = Point<int16_t>(243, 70);
		constexpr Point<int16_t> SFX_MUTE_BOX = Point<int16_t>(243, 100);
		constexpr Point<int16_t> SHAKE_BOX = Point<int16_t>(64, 249);
		constexpr Point<int16_t> WINDOW_BOX = Point<int16_t>(64, 304);
		constexpr Point<int16_t> FULLSCREEN_BOX = Point<int16_t>(64, 316);

		// The buttons stand at the right of the bar along the bottom.
		constexpr Point<int16_t> BT_OK = Point<int16_t>(192, 341);
		constexpr Point<int16_t> BT_CANCEL = Point<int16_t>(243, 341);
	}

	UISystemOption::UISystemOption() : UIDragElement<PosSYSTEMOPTION>()
	{
		nl::node SysOpt = NxFiles::ui_classic["UIWindow.img"]["SysOpt"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		Texture backgrnd = SysOpt["backgrnd"];
		sprites.emplace_back(SysOpt["backgrnd"]);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(Basic["BtOK2"], BT_OK);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], BT_CANCEL);

		box[false] = Basic["CheckBox"]["0"];
		box[true] = Basic["CheckBox"]["1"];

		Point<int16_t> box_size = box[false].get_dimensions();

		buttons[Buttons::BGM_MUTE] = std::make_unique<AreaButton>(BGM_MUTE_BOX, box_size);
		buttons[Buttons::SFX_MUTE] = std::make_unique<AreaButton>(SFX_MUTE_BOX, box_size);
		buttons[Buttons::SHAKE] = std::make_unique<AreaButton>(SHAKE_BOX, box_size);
		buttons[Buttons::WINDOW] = std::make_unique<AreaButton>(WINDOW_BOX, box_size);
		buttons[Buttons::FULLSCREEN] = std::make_unique<AreaButton>(FULLSCREEN_BOX, box_size);

		auto slider = [](SliderPlace place, std::function<void(int16_t)> onmoved = {})
		{
			return ClassicSlider(place.position, place.width, place.steps, onmoved);
		};

		auto volume_moved = [this](int16_t)
		{
			preview_volume();
		};

		sliders[Sliders::QUALITY] = slider(QUALITY_SLIDER);
		sliders[Sliders::BGM] = slider(BGM_SLIDER, volume_moved);
		sliders[Sliders::SFX] = slider(SFX_SLIDER, volume_moved);
		sliders[Sliders::MOUSE] = slider(MOUSE_SLIDER);
		sliders[Sliders::HP_ALERT] = slider(HP_SLIDER);
		sliders[Sliders::MP_ALERT] = slider(MP_SLIDER);

		sliders[Sliders::QUALITY].set_step(Setting<PictureQuality>::get().load());
		sliders[Sliders::BGM].set_step((Setting<BGMVolume>::get().load() + PERCENT_STEP / 2) / PERCENT_STEP);
		sliders[Sliders::SFX].set_step((Setting<SFXVolume>::get().load() + PERCENT_STEP / 2) / PERCENT_STEP);
		sliders[Sliders::MOUSE].set_step(Setting<MouseSpeed>::get().load());
		sliders[Sliders::HP_ALERT].set_step((Setting<HPAlert>::get().load() + PERCENT_STEP / 2) / PERCENT_STEP);
		sliders[Sliders::MP_ALERT].set_step((Setting<MPAlert>::get().load() + PERCENT_STEP / 2) / PERCENT_STEP);

		bgm_mute = Setting<BGMMute>::get().load();
		sfx_mute = Setting<SFXMute>::get().load();
		shake = Setting<ShakeScreen>::get().load();
		fullscreen = Setting<Fullscreen>::get().load();

		sliders[Sliders::BGM].set_enabled(!bgm_mute);
		sliders[Sliders::SFX].set_enabled(!sfx_mute);

		dimension = backgrnd.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);
	}

	void UISystemOption::draw(float inter) const
	{
		UIElement::draw_sprites(inter);

		for (const ClassicSlider &slider : sliders)
			slider.draw(position);

		box[bgm_mute].draw(position + BGM_MUTE_BOX);
		box[sfx_mute].draw(position + SFX_MUTE_BOX);
		box[shake].draw(position + SHAKE_BOX);
		box[!fullscreen].draw(position + WINDOW_BOX);
		box[fullscreen].draw(position + FULLSCREEN_BOX);

		UIElement::draw_buttons(inter);
	}

	void UISystemOption::remove_cursor()
	{
		UIDragElement::remove_cursor();

		for (ClassicSlider &slider : sliders)
			slider.remove_cursor();
	}

	Cursor::State UISystemOption::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (!dragged)
		{
			Point<int16_t> relative = cursorpos - position;

			for (ClassicSlider &slider : sliders)
				if (Cursor::State state = slider.send_cursor(relative, clicked); state != Cursor::State::IDLE)
					return state;
		}

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	void UISystemOption::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (escape)
				cancel();
			else if (keycode == KeyAction::Id::RETURN)
				button_pressed(Buttons::OK);
		}
	}

	UIElement::Type UISystemOption::get_type() const
	{
		return TYPE;
	}

	Button::State UISystemOption::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				save();
				deactivate();
				break;
			case Buttons::CANCEL:
				cancel();
				break;
			case Buttons::BGM_MUTE:
				bgm_mute = !bgm_mute;
				sliders[Sliders::BGM].set_enabled(!bgm_mute);
				preview_volume();
				break;
			case Buttons::SFX_MUTE:
				sfx_mute = !sfx_mute;
				sliders[Sliders::SFX].set_enabled(!sfx_mute);
				preview_volume();
				break;
			case Buttons::SHAKE:
				shake = !shake;
				break;
			case Buttons::WINDOW:
				fullscreen = false;
				break;
			case Buttons::FULLSCREEN:
				fullscreen = true;
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UISystemOption::save()
	{
		Setting<PictureQuality>::get().save(sliders[Sliders::QUALITY].get_step());
		Setting<BGMVolume>::get().save(sliders[Sliders::BGM].get_step() * PERCENT_STEP);
		Setting<SFXVolume>::get().save(sliders[Sliders::SFX].get_step() * PERCENT_STEP);
		Setting<MouseSpeed>::get().save(sliders[Sliders::MOUSE].get_step());
		Setting<HPAlert>::get().save(sliders[Sliders::HP_ALERT].get_step() * PERCENT_STEP);
		Setting<MPAlert>::get().save(sliders[Sliders::MP_ALERT].get_step() * PERCENT_STEP);
		Setting<BGMMute>::get().save(bgm_mute);
		Setting<SFXMute>::get().save(sfx_mute);
		Setting<ShakeScreen>::get().save(shake);

		// Saves the new mode itself.
		if (fullscreen != Setting<Fullscreen>::get().load())
			Window::get().toggle_fullscreen();
	}

	void UISystemOption::cancel()
	{
		Music::set_bgmvolume(Music::configured_volume());
		Sound::set_sfxvolume(Sound::configured_volume());

		deactivate();
	}

	void UISystemOption::preview_volume()
	{
		Music::set_bgmvolume(bgm_mute ? 0 : sliders[Sliders::BGM].get_step() * PERCENT_STEP);
		Sound::set_sfxvolume(sfx_mute ? 0 : sliders[Sliders::SFX].get_step() * PERCENT_STEP);
	}
}
