#include "PartySearchActions.h"

#include "UI.h"

#include "UITypes/UIPartySearch.h"
#include "UITypes/UIPartySearchIcon.h"

#include "../Net/Packets/GameplayPackets.h"

namespace ms
{
	namespace PartySearchActions
	{
		namespace
		{
			void show()
			{
				PartySearch::State state = PartySearch::get().state();

				if (state == PartySearch::State::IDLE)
				{
					if (auto icon = UI::get().get_element<UIPartySearchIcon>())
						icon->fade_out();
				}
				else
				{
					auto icon = UI::get().get_element<UIPartySearchIcon>();

					if (!icon)
					{
						UI::get().emplace<UIPartySearchIcon>();
						icon = UI::get().get_element<UIPartySearchIcon>();
					}

					if (icon)
						icon->show(state == PartySearch::State::SEARCHING);
				}

				if (auto window = UI::get().get_element<UIPartySearch>())
					window->refresh();
			}

			void apply(PartySearch::Send send)
			{
				const PartySearch::Wanted& wanted = PartySearch::get().wanted();

				switch (send)
				{
				case PartySearch::Send::START:
					PartySearchStartPacket(wanted.min_level, wanted.max_level, wanted.members, wanted.jobs).dispatch();
					break;
				case PartySearch::Send::STOP:
					PartySearchStopPacket().dispatch();
					break;
				case PartySearch::Send::NOTHING:
					break;
				}

				show();
			}
		}

		void start(const PartySearch::Wanted& wanted)
		{
			apply(PartySearch::get().start(wanted));
		}

		void resume()
		{
			apply(PartySearch::get().resume());
		}

		void pause()
		{
			apply(PartySearch::get().pause());
		}

		void stop()
		{
			apply(PartySearch::get().stop());
		}

		void leaving_channel()
		{
			PartySearch::get().leaving_channel();
		}

		void field_entered()
		{
			apply(PartySearch::get().field_entered());
		}

		void joined(size_t size)
		{
			apply(PartySearch::get().joined(size));
		}

		void reset()
		{
			PartySearch::get().reset();
			UI::get().remove(UIElement::Type::PARTYSEARCHICON);
		}
	}
}
