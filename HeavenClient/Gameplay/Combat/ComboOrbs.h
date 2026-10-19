#pragma once

#include <algorithm>
#include <array>
#include <cstdint>

namespace ms
{
	namespace combo
	{
		// Combo Attack's orbs around a character, as v83 draws them (FUN_0093cdbc): a center sprite 40
		// pixels over the feet, Combo Attack's under six orbs and Advanced Combo's from six, and ten orbs
		// on a ring of five slots, of which the newest five show.
		//
		// The ring turns once every 2 seconds and the center once a second, the other way (the
		// periods are -2000 and 1000). Like a drop's spin (period 300), a positive one is taken as
		// clockwise; not yet checked against period footage.
		class Orbs
		{
		public:
			// The layers: the two centers, then orbs 1 to 10.
			static constexpr size_t LAYERS = 12;
			static constexpr size_t CENTER = 0;
			static constexpr size_t ADVANCED_CENTER = 1;
			static constexpr int32_t MOST = 10;
			static constexpr int16_t RISE = 40;

			// The newest orb grows into place: from three times its size and a quarter seen to whole in
			// 300 ms (its frame's z0 300, z1 100, a0 64 and a1 255).
			static constexpr uint16_t POP = 300;

			static constexpr size_t orb(int32_t j)
			{
				return static_cast<size_t>(j) + 1;
			}

			// Where orb j sits from the center before the ring turns: 40 pixels out at j fifths of a
			// turn, cut to whole pixels. Orbs j and j + 5 share a slot.
			struct Offset
			{
				int16_t x;
				int16_t y;
			};

			static constexpr Offset slot(int32_t j)
			{
				constexpr Offset SLOTS[5] = { { 12, 38 }, { -32, 23 }, { -32, -23 }, { 12, -38 }, { 39, 0 } };

				return SLOTS[(j - 1) % 5];
			}

			// Orb j's sprite: Combo Attack's state/1 to 3 for the first five, Advanced Combo's state/3
			// to 5 for the rest.
			static constexpr bool advanced(int32_t j)
			{
				return j > 5;
			}

			static constexpr int32_t state(int32_t j)
			{
				return (j + 1) / 2;
			}

			// Show `orbs`, the Combo value less one, or -1 without it. A change animates: the newest
			// orb pops in, and losing them all (a finisher, or the first cast) fades every sprite out.
			// A character coming into view (`init`) shows what they have as it is.
			void set(int32_t orbs, bool init)
			{
				count = orbs;

				if (init && orbs == -1)
					return;

				if (!init && orbs == 0)
				{
					// One after another, a layer every 60 ms from 200, over again from the sixth.
					for (size_t c = 0; c < LAYERS; c++)
					{
						Layer &layer = layers[c];
						layer.fade_from = layer.faded();
						layer.fade_time = 0;
						layer.fade_end = static_cast<uint16_t>(200 + 60 * (c % 6));
					}

					return;
				}

				put(layers[CENTER], orbs >= 1 && orbs <= 5);
				put(layers[ADVANCED_CENTER], orbs > 5);

				for (int32_t j = 1; j <= MOST; j++)
				{
					Layer &layer = layers[orb(j)];

					if (j == orbs && !init)
					{
						layer.popping = true;
						layer.pop_time = 0;
					}

					if (orbs - 5 < j && j <= orbs)
						put(layer, true);
					else if (layer.popping)
						hide_later = true;
					else
						put(layer, false);
				}
			}

			// Move on by `ms` milliseconds. An orb that lost its place while popping in goes once it is
			// in (0x9329a2), and with it the others before it.
			void update(uint16_t ms)
			{
				time = static_cast<uint16_t>((time + ms) % 2000);

				for (Layer &layer : layers)
				{
					if (layer.popping)
					{
						layer.pop_time = std::min<uint16_t>(layer.pop_time + ms, POP);

						if (layer.pop_time == POP)
							layer.popping = false;
					}

					if (layer.fade_end > 0)
						layer.fade_time = std::min<uint16_t>(layer.fade_time + ms, layer.fade_end);
				}

				if (!hide_later)
					return;

				hide_later = false;

				for (int32_t j = 1; j <= MOST; j++)
				{
					Layer &layer = layers[orb(j)];

					if (layer.popping)
					{
						hide_later = true;
						break;
					}

					put(layer, false);
				}
			}

			// The orbs last set, or -1 without Combo.
			int32_t get_count() const
			{
				return count;
			}

			bool shown(size_t layer) const
			{
				return layers[layer].shown && layers[layer].opacity() > 0.0f;
			}

			float opacity(size_t layer) const
			{
				return layers[layer].shown ? layers[layer].opacity() : 0.0f;
			}

			float scale(size_t layer) const
			{
				const Layer &l = layers[layer];

				return l.popping ? 3.0f - 2.0f * l.pop_time / POP : 1.0f;
			}

			// The turns, in radians: positive is clockwise.
			float ring_angle() const
			{
				return -TURN * time / 2000;
			}

			float center_angle() const
			{
				return TURN * (time % 1000) / 1000;
			}

		private:
			static constexpr float TURN = 6.2831853f;

			struct Layer
			{
				bool shown = false;
				bool popping = false;
				uint16_t pop_time = 0;
				// A fade runs from fade_from to nothing over fade_end ms; 0 for none.
				float fade_from = 1.0f;
				uint16_t fade_time = 0;
				uint16_t fade_end = 0;

				float faded() const
				{
					if (fade_end == 0)
						return 1.0f;

					return fade_from * (1.0f - static_cast<float>(fade_time) / fade_end);
				}

				float opacity() const
				{
					float frame = popping ? (64.0f + 191.0f * pop_time / POP) / 255 : 1.0f;

					return faded() * frame;
				}
			};

			// Showing or hiding a layer ends its fade, so a faded orb shows again on the next gain (I).
			static void put(Layer &layer, bool shown)
			{
				layer.shown = shown;
				layer.fade_end = 0;
			}

			std::array<Layer, LAYERS> layers;
			bool hide_later = false;
			int32_t count = -1;
			uint16_t time = 0;
		};
	}
}
