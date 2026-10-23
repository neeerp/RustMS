#pragma once

#include "../UIElement.h"

#include "../ImageSlides.h"

#include "../../Graphics/Geometry.h"
#include "../../Graphics/Texture.h"

#include <string>
#include <vector>

namespace ms
{
	// OPEN_UI_STRING_LIST's (233) slideshow (v83 CUtilDlgEx type 9, report b098): a plain white 456 by 386 window
	// showing one image at a time, with Prev and Next to turn the pages. It is modal, and closing it sends nothing.
	class UIImageSlides : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::IMAGESLIDES;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `paths` name the pages' images, such as UI/UIWindow.img/QuestIcon/4/0.
		UIImageSlides(const std::vector<std::string> &paths);

		void draw(float alpha) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void show();

		enum Buttons : uint16_t
		{
			PREV,
			NEXT,
			OK,
			CLOSE
		};

		ColorBox background;
		std::vector<Texture> pages;
		uint16_t page;
		image_slides::Buttons shown;
	};
}
