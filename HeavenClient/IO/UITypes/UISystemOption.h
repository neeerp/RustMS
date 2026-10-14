#pragma once

#include <array>
#include <cstdint>

#include "../UIDragElement.h"

#include "../Components/ClassicSlider.h"

namespace ms
{
	// The v83 System Option window, UIWindow.img/SysOpt: sliders for picture
	// quality, music and sound volume, mouse speed and the HP and MP alerts,
	// with mute, screen shake and viewing mode boxes. The volumes change as
	// their sliders move; OK keeps everything, Cancel puts it back.
	class UISystemOption : public UIDragElement<PosSYSTEMOPTION>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SYSTEMOPTION;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UISystemOption();

		void draw(float inter) const override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void save();
		void cancel();
		void preview_volume();

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			BGM_MUTE,
			SFX_MUTE,
			SHAKE,
			WINDOW,
			FULLSCREEN
		};

		enum Sliders : uint16_t
		{
			QUALITY,
			BGM,
			SFX,
			MOUSE,
			HP_ALERT,
			MP_ALERT,
			NUM_SLIDERS
		};

		std::array<ClassicSlider, Sliders::NUM_SLIDERS> sliders;
		bool bgm_mute;
		bool sfx_mute;
		bool shake;
		bool fullscreen;
		Texture box[2];
	};
}
